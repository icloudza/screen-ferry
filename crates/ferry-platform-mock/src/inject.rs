//! 输入注入：`InputInject` 的测试实现。

use std::sync::{Arc, Mutex, MutexGuard};

use ferry_platform::{Error, InputInject, Result};
use ferry_trust::{InputAuthorization, InputProofVerifier};
use ferry_types::{InputEvent, InputProof, LockKeys};
use std::time::Instant;

/// 假的输入注入：记录所有注入的事件和锁定键同步。
#[derive(Clone, Default)]
pub struct MockInject {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    events: Vec<InputEvent>,
    synced_locks: Vec<LockKeys>,
    cursor_hidden: Vec<bool>,
    unavailable: bool,
    next_error: Option<Error>,
    next_cursor_error: Option<Error>,
    authorization: Option<InputAuthorization>,
    verifier: Option<InputProofVerifier>,
    verified: Vec<InputProof>,
    ended: u64,
}

impl MockInject {
    pub fn new() -> Self {
        Self::default()
    }

    /// 模拟独立的管理员授权；与用户配对列表无关。
    pub fn set_input_authorization(&self, authorization: Option<InputAuthorization>) {
        let mut state = self.lock();
        state.authorization = authorization;
        state.verifier = None;
    }

    pub fn verified_inputs(&self) -> Vec<InputProof> {
        self.lock().verified.clone()
    }
    pub fn ended_input_sessions(&self) -> u64 {
        self.lock().ended
    }

    /// 至今注入过的全部事件。
    pub fn events(&self) -> Vec<InputEvent> {
        self.lock().events.clone()
    }

    /// 每次同步锁定键时要求的状态（只含被同步的那些锁定键）。
    pub fn synced_locks(&self) -> Vec<LockKeys> {
        self.lock().synced_locks.clone()
    }

    /// 每次隐藏（`true`）或显示（`false`）本机光标的请求，按先后排列。
    pub fn cursor_hidden(&self) -> Vec<bool> {
        self.lock().cursor_hidden.clone()
    }

    /// 模拟锁屏、安全桌面或前台程序权限变化。
    pub fn set_available(&self, available: bool) {
        self.lock().unavailable = !available;
    }

    /// 下一次注入返回指定错误，之后恢复正常。
    pub fn fail_next(&self, error: Error) {
        self.lock().next_error = Some(error);
    }

    /// 下一次隐藏或显示光标失败，模拟窗口消息超时。
    pub fn fail_next_cursor_update(&self, error: Error) {
        self.lock().next_cursor_error = Some(error);
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl InputInject for MockInject {
    fn remote_challenge(&mut self, peer: [u8; 32]) -> Result<Option<[u8; 32]>> {
        let mut state = self.lock();
        let Some(authorization) = state
            .authorization
            .clone()
            .filter(|a| a.fingerprint.as_bytes() == &peer)
        else {
            return Ok(None);
        };
        let verifier = state.verifier.get_or_insert_with(|| {
            InputProofVerifier::new(authorization.public_key, Instant::now())
        });
        verifier
            .challenge(Instant::now())
            .map(Some)
            .map_err(|_| Error::PermissionDenied("模拟的安全输入租约到期"))
    }

    fn inject_verified(&mut self, proof: &InputProof) -> Result<()> {
        let mut state = self.lock();
        state
            .verifier
            .as_mut()
            .ok_or(Error::PermissionDenied("没有独立输入授权"))?
            .accept(proof, Instant::now())
            .map_err(|_| Error::PermissionDenied("输入证明无效"))?;
        state.verified.push(*proof);
        Ok(())
    }

    fn end_remote_input(&mut self) -> Result<()> {
        let mut state = self.lock();
        if state.verifier.take().is_some() {
            state.ended += 1;
        }
        Ok(())
    }

    fn check_available(&mut self) -> Result<()> {
        if self.lock().unavailable {
            Err(Error::PermissionDenied("测试中模拟的桌面不可用"))
        } else {
            Ok(())
        }
    }

    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        self.check_available()?;
        let mut state = self.lock();
        if state.verifier.is_some() {
            return Err(Error::PermissionDenied("不接受未签署输入"));
        }
        if let Some(error) = state.next_error.take() {
            return Err(error);
        }
        state.events.push(*event);
        Ok(())
    }

    fn sync_locks(&mut self, wanted: LockKeys, mask: LockKeys) -> Result<()> {
        self.lock().synced_locks.push(wanted & mask);
        Ok(())
    }

    fn set_cursor_hidden(&mut self, hidden: bool) -> Result<()> {
        let mut state = self.lock();
        state.cursor_hidden.push(hidden);
        if let Some(error) = state.next_cursor_error.take() {
            return Err(error);
        }
        Ok(())
    }
}
