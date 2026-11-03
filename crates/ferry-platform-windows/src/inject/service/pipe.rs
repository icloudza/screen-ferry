//! 一实例的异步本机管道。总期限不因半帧延长；取消完成前绝不释放 OVERLAPPED 或缓冲区。

use std::time::{Duration, Instant};

use ferry_platform::{Error, Result};
use ferry_proto::input_agent::FRAME_LEN;
use windows::Win32::Foundation::{
    ERROR_IO_PENDING, ERROR_NO_DATA, ERROR_OPERATION_ABORTED, ERROR_PIPE_CONNECTED, GENERIC_READ,
    GENERIC_WRITE, WAIT_FAILED, WAIT_OBJECT_0,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, OPEN_EXISTING,
    PIPE_ACCESS_DUPLEX, ReadFile, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT, WriteFile,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForMultipleObjects};
use windows::core::{HSTRING, PCWSTR};

use super::security::Descriptor;
use super::util::{Handle, failure, os};

pub(super) const IO_WAIT: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub(super) struct Pipe {
    pub handle: Handle,
    event: Handle,
}

pub(super) fn name(session: u32) -> HSTRING {
    HSTRING::from(format!(r"\\.\pipe\ScreenFerry.Input.v1.{session}"))
}

impl Pipe {
    fn new(handle: Handle) -> Result<Self> {
        // SAFETY: 每条管道独占一个未命名手动复位事件；操作串行，不共享 OVERLAPPED。
        let event = Handle::new(
            unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
                .map_err(|e| os("创建管道完成事件", e))?,
        )?;
        Ok(Self { handle, event })
    }

    pub fn create(session: u32, user: &str) -> Result<Self> {
        let security = Descriptor::from_sddl(&format!("D:P(A;;GA;;;SY)(A;;GRGW;;;{user})"))?;
        let attributes = security.attributes(false);
        // SAFETY: 固定本机名称，明确 ACL，一实例、禁止远程访问、禁止占用已存在的管道名。
        let handle = unsafe {
            CreateNamedPipeW(
                &name(session),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                4096,
                4096,
                0,
                Some(&attributes),
            )
        };
        Self::new(Handle::new(handle)?)
    }

    pub fn connect(session: u32) -> Result<Self> {
        // SAFETY: 固定本机路径，SECURITY_IDENTIFICATION 禁止对端冒充当前用户。
        let handle = unsafe {
            CreateFileW(
                &name(session),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                Default::default(),
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                None,
            )
        }
        .map_err(|e| os("连接输入代理", e))?;
        Self::new(Handle::new(handle)?)
    }

    fn overlapped(&self) -> Result<OVERLAPPED> {
        // SAFETY: 上一个操作已完成或已取消并确认完成，才能复用事件。
        unsafe { ResetEvent(self.event.0) }.map_err(|e| os("重置管道事件", e))?;
        Ok(OVERLAPPED {
            hEvent: self.event.0,
            ..Default::default()
        })
    }

    pub fn accept(&self) -> Result<bool> {
        let mut operation = self.overlapped()?;
        // SAFETY: operation 存活到完成/取消完成；一条管道只接受一个对端。
        let started = unsafe { ConnectNamedPipe(self.handle.0, Some(&mut operation)) };
        if let Err(e) = &started {
            if e.code() == ERROR_PIPE_CONNECTED.to_hresult() {
                return Ok(true);
            }
            if e.code() == ERROR_NO_DATA.to_hresult() {
                self.disconnect();
                return Ok(false);
            }
        }
        let connected = self
            .complete(
                started,
                &mut operation,
                Instant::now() + Duration::from_millis(100),
                None,
            )?
            .is_some();
        if !connected {
            self.disconnect();
        }
        Ok(connected)
    }

    pub fn disconnect(&self) {
        // SAFETY: 保留服务端句柄和管道名称，清空旧连接，防止名称被其他用户抢占。
        let _ = unsafe { DisconnectNamedPipe(self.handle.0) };
    }

    pub fn read(
        &self,
        deadline: Instant,
        stop: Option<&Handle>,
    ) -> Result<Option<[u8; FRAME_LEN]>> {
        let mut frame = [0; FRAME_LEN];
        let mut offset = 0;
        while offset < FRAME_LEN {
            let mut operation = self.overlapped()?;
            // SAFETY: 缓冲区及 operation 始终存活到 complete 确认内核不再访问它们。
            let started = unsafe {
                ReadFile(
                    self.handle.0,
                    Some(&mut frame[offset..]),
                    None,
                    Some(&mut operation),
                )
            };
            match self.complete(started, &mut operation, deadline, stop)? {
                Some(0) => return Err(failure("输入代理连接已关闭")),
                Some(read) => offset += read as usize,
                None if offset == 0 => return Ok(None),
                None => return Err(failure("输入代理收到不完整的帧")),
            }
        }
        Ok(Some(frame))
    }

    pub fn write(&self, frame: &[u8; FRAME_LEN], deadline: Instant) -> Result<()> {
        let mut offset = 0;
        while offset < FRAME_LEN {
            let mut operation = self.overlapped()?;
            // SAFETY: 同 read。固定大小的输出不会因不读取的对端无限挂起。
            let started = unsafe {
                WriteFile(
                    self.handle.0,
                    Some(&frame[offset..]),
                    None,
                    Some(&mut operation),
                )
            };
            match self.complete(started, &mut operation, deadline, None)? {
                Some(0) | None => return Err(failure("输入代理写入超时或连接已关闭")),
                Some(written) => offset += written as usize,
            }
        }
        Ok(())
    }

    fn complete(
        &self,
        started: windows::core::Result<()>,
        operation: &mut OVERLAPPED,
        deadline: Instant,
        stop: Option<&Handle>,
    ) -> Result<Option<u32>> {
        if let Err(e) = &started
            && e.code() != ERROR_IO_PENDING.to_hresult()
        {
            return Err(os("输入代理管道 I/O", e.clone()));
        }
        let mut cancelled = false;
        let mut wait_failed = false;
        if started.is_err() {
            let mut events = vec![self.event.0];
            if let Some(stop) = stop {
                events.push(stop.0);
            }
            let ms = deadline
                .saturating_duration_since(Instant::now())
                .as_millis()
                .min(u128::from(u32::MAX - 1)) as u32;
            // SAFETY: 有效事件句柄，等待不超过请求剩余期限，同时响应服务停止事件。
            let result = unsafe { WaitForMultipleObjects(&events, false, ms) };
            if result != WAIT_OBJECT_0 {
                cancelled = true;
                wait_failed = result == WAIT_FAILED;
                // SAFETY: 必须指定同一个 OVERLAPPED，随后等待取消确认，不能提前释放缓冲区。
                let _ = unsafe { CancelIoEx(self.handle.0, Some(operation)) };
            }
        }
        let mut count = 0;
        // SAFETY: 即使 CancelIoEx 返回，也必须确认操作完成后才可复用/释放 operation 和缓冲区。
        // 只对本机 NPFS 请求调用，不访问远程管道或第三方设备驱动。
        let result = unsafe { GetOverlappedResult(self.handle.0, operation, &mut count, true) };
        if stop.is_some_and(Handle::signalled) {
            return Err(Error::Cancelled);
        }
        if wait_failed {
            return Err(failure("输入代理等待 I/O 失败"));
        }
        match result {
            Ok(()) => Ok(Some(count)),
            Err(e) if cancelled && e.code() == ERROR_OPERATION_ABORTED.to_hresult() => Ok(None),
            Err(e) => Err(os("完成输入代理 I/O", e)),
        }
    }
}
