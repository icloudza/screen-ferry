//! 普通后台的注入门面。已启用服务时，服务故障不能静默降级到错误的桌面。

use std::time::Instant;

use ferry_platform::{Error, InputInject, Result};
use ferry_proto::input_agent::{Command, Request, Response, Status};
use ferry_types::{InputEvent, InputProof, LockKeys};

use super::controller;
use super::pipe::{IO_WAIT, Pipe};
use super::security::{self, Installed};
use super::util::{Handle, failure, ticks};
use crate::inject::LocalInject;

/// 根据已安装服务的状态选择本地注入或特权输入代理。
pub struct WinInject {
    local: LocalInject,
    privileged: bool,
    connection: Option<Client>,
    bound: Option<[u8; 32]>,
}

struct Client {
    pipe: Pipe,
    sequence: u64,
    _peer: Handle,
    _installed: Installed,
}

impl WinInject {
    pub fn new() -> Self {
        Self {
            local: LocalInject::new(),
            // 安装时选了安全输入、服务在运行、服务程序受保护，才走特权代理；否则就是普通权限注入。
            privileged: controller::running() && Installed::registered().is_ok(),
            connection: None,
            bound: None,
        }
    }

    fn request(&mut self, command: Command) -> Result<Response> {
        if self.connection.is_none() {
            if self.bound.is_some() {
                return Err(failure("安全输入管道已关闭，必须重建远端会话"));
            }
            self.connection = Some(Client::connect()?);
        }
        let result = self
            .connection
            .as_mut()
            .expect("刚建立连接")
            .request(command);
        if result
            .as_ref()
            .is_err_and(|e| !matches!(e, Error::Unsupported(_)))
        {
            // 超时或无效回复后绝不在旧连接继续，也不重放未确认的输入。
            self.connection = None;
        }
        result
    }
}

impl Default for WinInject {
    fn default() -> Self {
        Self::new()
    }
}

impl InputInject for WinInject {
    fn remote_challenge(&mut self, peer: [u8; 32]) -> Result<Option<[u8; 32]>> {
        if !self.privileged {
            return Ok(None);
        }
        let command = if self.bound == Some(peer) {
            Command::Check
        } else {
            if self.bound.is_some() {
                return Err(failure("安全输入管道不能更换远端身份"));
            }
            Command::Bind { peer }
        };
        let response = self.request(command)?;
        if response.status == Status::Unauthorized {
            self.connection = None;
            return Ok(None);
        }
        let challenge = response
            .challenge
            .ok_or_else(|| failure("代理没有提供远端输入挑战"))?;
        self.bound = Some(peer);
        Ok(Some(challenge))
    }

    fn inject_verified(&mut self, proof: &InputProof) -> Result<()> {
        if self.bound.is_none() {
            return Err(failure("没有管理员授权的安全输入会话"));
        }
        self.request(Command::Verified(*proof)).map(|_| ())
    }

    fn end_remote_input(&mut self) -> Result<()> {
        let result = if self.bound.is_some() && self.connection.is_some() {
            self.request(Command::Release).map(|_| ())
        } else {
            Ok(())
        };
        self.connection = None;
        self.bound = None;
        result
    }

    fn check_available(&mut self) -> Result<()> {
        // 只有绑定了授权过的远端才经过代理。装了服务但这台控制端没被授权时，和没装服务完全一样：
        // 否则每次检查都会另开一条管道连接，代理 1.5 秒收不到下一条请求就把它关掉，下一次检查必然失败。
        if self.bound.is_some() {
            self.request(Command::Check).map(|_| ())
        } else {
            self.local.check_available()
        }
    }

    fn local_pointer(&mut self) -> Option<(ferry_types::Point, bool)> {
        self.local.local_pointer()
    }

    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        if self.bound.is_some() {
            return Err(Error::PermissionDenied("安全输入必须原样转发远端签署动作"));
        }
        self.local.inject(event)
    }

    fn sync_locks(&mut self, wanted: LockKeys, mask: LockKeys) -> Result<()> {
        if self.bound.is_some() {
            return Err(Error::PermissionDenied(
                "安全锁定键必须包含在签署的进入动作中",
            ));
        }
        self.local.sync_locks(wanted, mask)
    }

    fn set_cursor_hidden(&mut self, hidden: bool) -> Result<()> {
        if self.bound.is_some() {
            return Err(Error::PermissionDenied("安全桌面光标必须由签署动作管理"));
        }
        self.local.set_cursor_hidden(hidden)
    }
}

impl Drop for WinInject {
    fn drop(&mut self) {
        if let Some(connection) = &mut self.connection {
            let _ = connection.request(Command::Release);
        }
        // 关闭管道后，代理也会清理全部属于这条连接的输入状态。
    }
}

impl Client {
    fn connect() -> Result<Self> {
        let installed = Installed::registered()?;
        let session = security::current_session()?;
        let pipe = Pipe::connect(session)?;
        let peer = security::verify_server(pipe.handle.0, &installed, session)?;
        let mut connection = Self {
            pipe,
            sequence: 0,
            _peer: peer,
            _installed: installed,
        };
        connection.request(Command::Check)?;
        Ok(connection)
    }

    fn request(&mut self, command: Command) -> Result<Response> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| failure("输入代理序号耗尽"))?;
        let deadline = Instant::now() + IO_WAIT;
        let request = Request {
            sequence: self.sequence,
            expires_ms: ticks() + 400,
            command,
        };
        let frame = request.encode().map_err(|e| failure(&e.to_string()))?;
        self.pipe.write(&frame, deadline)?;
        let frame = self
            .pipe
            .read(deadline, None)?
            .ok_or_else(|| failure("输入代理响应超时"))?;
        let response = Response::decode(&frame).map_err(|e| failure(&e.to_string()))?;
        if response.sequence != self.sequence {
            return Err(failure("输入代理回复序号不匹配"));
        }
        match response.status {
            Status::Ok | Status::Unauthorized => Ok(response),
            Status::Unsupported => Err(Error::Unsupported("输入代理不支持这个键鼠事件")),
            Status::Unavailable => Err(Error::PermissionDenied("Windows 当前输入桌面暂时不可用")),
            Status::Expired => Err(failure("输入请求已过期，已归还本机控制")),
            Status::Failed => Err(failure("Windows 输入代理执行失败，已归还本机控制")),
        }
    }
}
