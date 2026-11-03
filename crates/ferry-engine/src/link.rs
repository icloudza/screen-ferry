//! 服务端与客户端共用的连接参数和收发帮助函数。

use std::time::Duration;

use ferry_net::Framed;
use ferry_proto::{Capabilities, Message};
use ferry_types::Display;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;

use crate::error::{Error, Result};

/// 默认端口。
pub const DEFAULT_PORT: u16 = 24810;

/// 本机声明的可选能力。`command_key`：本机的快捷键以 Command 键为主。
pub(crate) fn capabilities(clipboard: bool, command_key: bool) -> Capabilities {
    let mut caps = Capabilities::FILES
        | Capabilities::EDGE_DROP
        | Capabilities::REMOTE_SETTINGS
        | Capabilities::APP_INFO
        | Capabilities::SECURE_INPUT;
    if clipboard {
        caps |= Capabilities::CLIPBOARD;
    }
    if command_key {
        caps |= Capabilities::COMMAND_KEY;
    }
    caps
}

/// 本机的程序版本，握手后经 `AppInfo` 告诉对方。
pub(crate) fn app_info() -> Message {
    Message::AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// 对方是什么系统：快捷键以 Command 为主的是 macOS，其余目前都是 Windows。
pub(crate) fn platform_of(command_key: bool) -> String {
    if command_key { "macos" } else { "windows" }.to_owned()
}

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// 整数矩形转成浮层用的浮点矩形。
pub(crate) fn frame_of(rect: ferry_types::Rect) -> ferry_types::Frame {
    ferry_types::Frame::new(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
}

/// 会话期间每隔这么久检查一次本机的显示器布局（改分辨率、缩放、插拔显示器），变了就告诉对方。
/// 查询一次只要几微秒。
pub(crate) const DISPLAY_CHECK: Duration = Duration::from_secs(1);

/// 单条消息的发送上限。对方卡死时不至于把本机一起卡住。
const SEND_TIMEOUT: Duration = Duration::from_millis(500);

/// 带超时地发送一条消息。
pub(crate) async fn send<S>(framed: &mut Framed<S>, message: &Message) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    timeout(SEND_TIMEOUT, framed.send(message))
        .await
        .map_err(|_| Error::Timeout("发送"))??;
    Ok(())
}

/// 主显示器的缩放比例；没有主显示器时取第一块。
pub(crate) fn primary_scale(displays: &[Display]) -> u16 {
    displays
        .iter()
        .find(|d| d.primary)
        .or(displays.first())
        .map_or(100, |d| d.scale_percent.max(1))
}
