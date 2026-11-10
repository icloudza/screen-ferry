//! 当前控制台会话中的 SYSTEM 输入代理。每个桌面独占线程，网络与文件处理不进入这里。

use std::collections::BTreeMap;
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::policy::Policy;
use ferry_core::{InputLease, Pressed};
use ferry_platform::{Error, InputInject, Result};
use ferry_proto::SecureAction;
use ferry_proto::input_agent::{Command, Request, Response, Status};
use ferry_trust::InputProofVerifier;
use ferry_types::{InputEvent, Point};
use windows::Win32::Foundation::{DuplicateHandle, HANDLE};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CREATEWINDOW, DESKTOP_READOBJECTS,
    DESKTOP_WRITEOBJECTS, GetUserObjectInformationW, HDESK, OpenInputDesktop, SetThreadDesktop,
    UOI_NAME,
};
use windows::Win32::System::Threading::GetCurrentProcess;

use super::controller::system_environment;
use super::pipe::Pipe;
use super::security::{self, Installed};
use super::util::{Handle, failure, os, ticks};
use crate::inject::LocalInject;

/// 仅 SYSTEM 可运行。核验父服务后，将停止事件按最小等待权限复制到本会话。
pub fn run_agent(parent_pid: u32, stop_handle: usize) -> Result<()> {
    system_environment()?;
    let installed = Installed::load()?;
    let session = security::current_session()?;
    if session == 0 || session != security::active_session() {
        return Err(failure("输入代理不在当前控制台会话"));
    }
    let user = security::session_user(session)?;
    security::drop_agent_privileges()?;
    let parent = security::service_parent(parent_pid, &installed)?;
    let mut copied = HANDLE::default();
    // SAFETY: 来源是已核验的 Session 0 SYSTEM 服务；只复制事件的 SYNCHRONIZE 权限。
    // 不使用跨会话句柄继承，不复制事件修改、令牌或任意文件的权限。
    unsafe {
        DuplicateHandle(
            parent.0,
            HANDLE(stop_handle as *mut _),
            GetCurrentProcess(),
            &mut copied,
            0x0010_0000,
            false,
            Default::default(),
        )
    }
    .map_err(|e| os("取得服务停止事件", e))?;
    let stop = Handle::new(copied)?;
    let pipe = Pipe::create(session, &user)?;
    let mut desks = Desks::default();
    let result = serve(&pipe, &stop, session, &user, &mut desks);
    let cleanup = desks.shutdown();
    result.and(cleanup)
}

fn serve(pipe: &Pipe, stop: &Handle, session: u32, user: &str, desks: &mut Desks) -> Result<()> {
    while !stop.signalled() && security::active_session() == session {
        // 即使后台已断开，也在原桌面恢复时补发尚未成功释放的键。
        let _ = desks.refresh();
        if !pipe.accept()? {
            continue;
        }
        if let Ok(peer) = security::verify_client(pipe.handle.0, session, user) {
            let _ = connection(pipe, stop, &peer, session, user, desks);
        }
        desks.hidden = false;
        desks.release()?;
        pipe.disconnect();
    }
    Ok(())
}

fn connection(
    pipe: &Pipe,
    stop: &Handle,
    peer: &Handle,
    session: u32,
    user: &str,
    desks: &mut Desks,
) -> Result<()> {
    let mut lease = InputLease::new(Instant::now());
    let mut authority: Option<Authority> = None;
    loop {
        if stop.signalled()
            || peer.signalled()
            || security::active_session() != session
            || lease.expired(Instant::now())
        {
            return Ok(());
        }
        if let Some(authority) = &authority {
            authority.check()?;
        }
        let _ = desks.refresh();
        let Some(frame) = pipe.read(Instant::now() + Duration::from_millis(100), Some(stop))?
        else {
            continue;
        };
        let request = Request::decode(&frame).map_err(|e| failure(&e.to_string()))?;
        if !lease.accept(
            request.sequence,
            request.expires_ms,
            ticks(),
            Instant::now(),
        ) {
            return Err(failure("输入请求过期、重放或租约已到期"));
        }
        let mut challenge = None;
        let result = match request.command {
            Command::Bind { peer } if authority.is_none() => {
                if let Some(policy) = Policy::load(user, peer)? {
                    let mut verifier = InputProofVerifier::new(policy.public_key(), Instant::now());
                    challenge = Some(
                        verifier
                            .challenge(Instant::now())
                            .map_err(|e| failure(&e.to_string()))?,
                    );
                    authority = Some(Authority {
                        policy,
                        verifier,
                        entered: false,
                    });
                    Ok(())
                } else {
                    pipe.write(
                        &Response {
                            sequence: request.sequence,
                            status: Status::Unauthorized,
                            challenge: None,
                        }
                        .encode(),
                        Instant::now() + Duration::from_millis(100),
                    )?;
                    return Ok(());
                }
            }
            Command::Check => {
                if let Some(authority) = &mut authority {
                    challenge = Some(
                        authority
                            .verifier
                            .challenge(Instant::now())
                            .map_err(|e| failure(&e.to_string()))?,
                    );
                }
                desks.execute(Command::Check, request.expires_ms)
            }
            Command::Verified(proof) => {
                let authority = authority
                    .as_mut()
                    .ok_or_else(|| failure("尚未绑定管理员授权的远端"))?;
                authority.check()?;
                authority
                    .verifier
                    .accept(&proof, Instant::now())
                    .map_err(|e| failure(&e.to_string()))?;
                let action =
                    SecureAction::decode(&proof.body).map_err(|e| failure(&e.to_string()))?;
                secure_action(desks, &mut authority.entered, action, request.expires_ms)
            }
            Command::Release => {
                desks.hidden = false;
                desks.release()
            }
            // 这些变体只供桌面工作线程内部使用。管道客户端绝不能直接调用。
            _ => Err(Error::PermissionDenied("特权输入必须携带远端设备签名")),
        };
        let status = match result {
            Ok(()) => Status::Ok,
            Err(Error::Unsupported(_)) => Status::Unsupported,
            Err(Error::PermissionDenied(_)) => Status::Unavailable,
            Err(e) => return Err(e),
        };
        if status != Status::Ok {
            challenge = None;
        }
        pipe.write(
            &Response {
                sequence: request.sequence,
                status,
                challenge,
            }
            .encode(),
            Instant::now() + Duration::from_millis(100),
        )?;
        if status == Status::Unavailable || request.command == Command::Release {
            return Ok(());
        }
    }
}

struct Authority {
    policy: Policy,
    verifier: InputProofVerifier,
    entered: bool,
}
impl Authority {
    fn check(&self) -> Result<()> {
        if self.verifier.expired(Instant::now()) || !self.policy.valid()? {
            return Err(failure("远端输入授权已撤销或租约到期"));
        }
        Ok(())
    }
}

fn secure_action(
    desks: &mut Desks,
    entered: &mut bool,
    action: SecureAction,
    expires: u64,
) -> Result<()> {
    match action {
        SecureAction::Alive => {
            desks.execute(Command::Check, expires)?;
            // 只藏一次。保活每 250 毫秒来一条，每次都藏会把光标反复挪回屏幕中央，
            // 这台电脑的人就没法用自己的鼠标了；藏好之后他一动鼠标，光标照常出现。
            if !*entered && !desks.hidden {
                desks.execute(Command::Cursor { hidden: true }, expires)?;
            }
        }
        SecureAction::Enter(enter) => {
            desks.release()?;
            desks.execute(
                Command::Input(InputEvent::PointerMotion {
                    position: enter.position,
                }),
                expires,
            )?;
            desks.execute(
                Command::Locks {
                    wanted: enter.locks,
                    mask: ferry_types::LockKeys::CAPS,
                },
                expires,
            )?;
            desks.execute(Command::Cursor { hidden: false }, expires)?;
            *entered = true;
        }
        SecureAction::Leave { .. } => {
            desks.release()?;
            desks.execute(Command::Cursor { hidden: true }, expires)?;
            *entered = false;
        }
        SecureAction::Input(event) => {
            if !*entered {
                return Err(failure("收到进入屏幕前的签署输入"));
            }
            if let InputEvent::Button { position, .. } | InputEvent::Wheel { position, .. } = event
            {
                desks.execute(
                    Command::Input(InputEvent::PointerMotion { position }),
                    expires,
                )?;
            }
            match desks.execute(Command::Input(event), expires) {
                Ok(()) | Err(Error::Unsupported(_)) => {}
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
}

struct Desktop(HDESK);
// SAFETY: OpenInputDesktop 返回的句柄尚未绑定线程，只将所有权交给一个工作线程。
unsafe impl Send for Desktop {}
impl Drop for Desktop {
    fn drop(&mut self) {
        // SAFETY: 只关闭 OpenInputDesktop 的拥有句柄，不关闭 GetThreadDesktop 的借用句柄。
        let _ = unsafe { CloseDesktop(self.0) };
    }
}

fn input_desktop() -> Result<(&'static str, Desktop)> {
    // SAFETY: 仅打开当前输入桌面，不切换用户看到的桌面，不请求创建其他桌面。
    let desktop = Desktop(
        unsafe {
            OpenInputDesktop(
                Default::default(),
                false,
                DESKTOP_ACCESS_FLAGS(
                    DESKTOP_READOBJECTS.0 | DESKTOP_WRITEOBJECTS.0 | DESKTOP_CREATEWINDOW.0,
                ),
            )
        }
        .map_err(|_| Error::PermissionDenied("Windows 输入桌面正在切换"))?,
    );
    let mut name = [0_u16; 64];
    // SAFETY: 完整的固定大小 UTF-16 输出缓冲区，长度以字节传入。
    unsafe {
        GetUserObjectInformationW(
            HANDLE(desktop.0.0),
            UOI_NAME,
            Some(name.as_mut_ptr().cast()),
            size_of_val(&name) as u32,
            None,
        )
    }
    .map_err(|e| os("读取输入桌面名称", e))?;
    let length = name
        .iter()
        .position(|&c| c == 0)
        .ok_or_else(|| failure("输入桌面名称过长"))?;
    let name = String::from_utf16_lossy(&name[..length]);
    let name = if name.eq_ignore_ascii_case("Default") {
        "Default"
    } else if name.eq_ignore_ascii_case("Winlogon") {
        "Winlogon"
    } else {
        return Err(Error::PermissionDenied("不支持的 Windows 输入桌面"));
    };
    Ok((name, desktop))
}

#[derive(Default)]
struct Desks {
    workers: BTreeMap<&'static str, Worker>,
    current: Option<&'static str>,
    hidden: bool,
}

impl Desks {
    fn refresh(&mut self) -> Result<()> {
        let (name, desktop) = input_desktop()?;
        if self.current == Some(name) {
            return Ok(());
        }
        if let Some(previous) = self.current {
            let worker = &self.workers[previous];
            let _ = worker.request(Command::Release, ticks() + 300);
            worker.request(Command::Cursor { hidden: false }, ticks() + 300)?;
        }
        if !self.workers.contains_key(name) {
            self.workers.insert(name, Worker::start(desktop)?);
        }
        let worker = &self.workers[name];
        // 返回旧桌面时，先释放那次切换中未成功释放的键，再接受新输入。
        worker.request(Command::Release, ticks() + 300)?;
        worker.request(
            Command::Cursor {
                hidden: self.hidden,
            },
            ticks() + 300,
        )?;
        self.current = Some(name);
        Ok(())
    }

    fn execute(&mut self, command: Command, expires: u64) -> Result<()> {
        if ticks() >= expires {
            return Err(failure("输入代理请求已到期"));
        }
        if command == Command::Release {
            self.hidden = false;
            return self.release();
        }
        self.refresh()?;
        self.workers[self.current.expect("已选择桌面")].request(command, expires)?;
        if let Command::Cursor { hidden } = command {
            self.hidden = hidden;
        }
        Ok(())
    }

    fn release(&mut self) -> Result<()> {
        for worker in self.workers.values() {
            match worker.request(Command::Release, ticks() + 300) {
                Ok(()) | Err(Error::PermissionDenied(_) | Error::Os { .. }) => {}
                Err(e) => return Err(e),
            }
            worker.request(Command::Cursor { hidden: false }, ticks() + 300)?;
        }
        // 同一个桌面再次变为活动时也要先重试待释放状态。
        self.current = None;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.hidden = false;
        let result = self.release();
        self.workers.clear();
        result
    }
}

struct Work {
    command: Command,
    expires: u64,
    reply: SyncSender<Result<()>>,
}
struct Worker {
    sender: SyncSender<Work>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn start(desktop: Desktop) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Work>(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("ferry-secure-input".into())
            .spawn(move || {
                // SAFETY: 新线程尚无窗口、钩子或消息队列，先绑定桌面；句柄在整个线程生命周期有效。
                let bound =
                    unsafe { SetThreadDesktop(desktop.0) }.map_err(|e| os("绑定输入线程桌面", e));
                let succeeded = bound.is_ok();
                let _ = ready_tx.send(bound);
                if !succeeded {
                    return;
                }
                let mut input = OwnedInput {
                    inject: LocalInject::new(),
                    pressed: Pressed::new(),
                    at: Point::default(),
                };
                while let Ok(work) = receiver.recv() {
                    let result = if ticks() >= work.expires {
                        Err(failure("输入请求在执行前已到期"))
                    } else {
                        input.execute(work.command)
                    };
                    let _ = work.reply.send(result);
                }
                let _ = input.release();
                let _ = input.inject.set_cursor_hidden(false);
                drop(input);
                drop(desktop);
            })
            .map_err(|e| failure(&e.to_string()))?;
        ready_rx
            .recv_timeout(Duration::from_millis(300))
            .map_err(|_| failure("输入线程启动超时"))??;
        Ok(Self {
            sender,
            thread: Some(thread),
        })
    }

    fn request(&self, command: Command, expires: u64) -> Result<()> {
        let (reply, receive) = mpsc::sync_channel(1);
        self.sender
            .try_send(Work {
                command,
                expires,
                reply,
            })
            .map_err(|_| failure("输入工作线程繁忙或已退出"))?;
        let wait = Duration::from_millis(expires.saturating_sub(ticks()).min(300));
        receive
            .recv_timeout(wait)
            .map_err(|_| failure("输入工作线程响应超时"))?
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // 不等待可能卡在 Win32 中的线程。代理随后退出，其 Job 和服务负责回收进程。
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished)
            && let Some(thread) = self.thread.take()
        {
            let _ = thread.join();
        }
    }
}

struct OwnedInput {
    inject: LocalInject,
    pressed: Pressed,
    at: Point,
}

impl OwnedInput {
    fn execute(&mut self, command: Command) -> Result<()> {
        if let Command::Cursor { hidden } = command {
            return self.inject.set_cursor_hidden(hidden);
        }
        self.inject.check_available()?;
        match command {
            Command::Check => Ok(()),
            Command::Input(event) => {
                self.inject.inject(&event)?;
                self.pressed.track(&event);
                if let InputEvent::PointerMotion { position } = event {
                    self.at = position;
                }
                Ok(())
            }
            Command::Locks { wanted, mask } => self.inject.sync_locks(wanted, mask),
            Command::Release => self.release(),
            Command::Cursor { .. } => unreachable!("已处理光标命令"),
            Command::Bind { .. } | Command::Verified(_) => Err(failure("认证命令不能传入桌面线程")),
        }
    }

    fn release(&mut self) -> Result<()> {
        if self.pressed.is_empty() {
            return Ok(());
        }
        self.inject.check_available()?;
        let mut result = Ok(());
        for event in self.pressed.clone().release_all(self.at) {
            match self.inject.inject(&event) {
                Ok(()) => self.pressed.track(&event),
                Err(e) => result = Err(e),
            }
        }
        result
    }
}
