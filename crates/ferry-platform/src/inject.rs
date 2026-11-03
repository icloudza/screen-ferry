//! 输入注入：定义 `InputInject` trait。

use ferry_types::{InputEvent, InputProof, LockKeys};

use crate::error::Result;

/// 向本机注入键鼠事件。
pub trait InputInject: Send {
    /// 按管理员的独立授权绑定远端，返回代理生成的随机挑战；普通注入平台返回 None。
    fn remote_challenge(&mut self, _peer: [u8; 32]) -> Result<Option<[u8; 32]>> {
        Ok(None)
    }

    /// 原样交给代理验证；调用方不能先验证再把未认证的动作送进特权通道。
    fn inject_verified(&mut self, _proof: &InputProof) -> Result<()> {
        Err(crate::Error::Unsupported("没有安全输入代理"))
    }

    /// 本机只可撤销、释放状态，不能借清理接口新增按键或隐藏光标。
    fn end_remote_input(&mut self) -> Result<()> {
        Ok(())
    }

    /// 检查当前桌面和前台程序是否接受输入。运行中也会检查，暂时不可用时结束会话，
    /// 让控制端收回键鼠；恢复后可以重新连接。无需额外检查的平台使用默认实现。
    fn check_available(&mut self) -> Result<()> {
        Ok(())
    }

    /// 注入一个事件。`Button` 与 `Wheel` 只执行按键和滚动，光标定位由调用方先行注入。
    fn inject(&mut self, event: &InputEvent) -> Result<()>;

    /// 把 `mask` 中的锁定键调整到与 `wanted` 一致，其余锁定键不动。
    fn sync_locks(&mut self, wanted: LockKeys, mask: LockKeys) -> Result<()>;

    /// 隐藏或重新显示本机光标。光标在对方屏幕上时隐藏，免得本机屏幕上还停着一个不动的光标；
    /// 本机用户自己动一下鼠标，光标就应当重新出现。不支持的平台什么也不做。
    fn set_cursor_hidden(&mut self, hidden: bool) -> Result<()> {
        let _ = hidden;
        Ok(())
    }
}
