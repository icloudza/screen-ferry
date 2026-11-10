//! 网络错误。

use ferry_proto::{DecodeError, EncodeError, Reject};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("网络错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("无法发送：{0}")]
    Encode(#[from] EncodeError),
    #[error("对方违反协议：{0}")]
    Decode(#[from] DecodeError),
    #[error("连接在一帧中途断开")]
    TruncatedFrame,
    #[error("连接已关闭")]
    Closed,
    #[error("握手时应收到 {expected}，却收到了其他消息")]
    UnexpectedMessage { expected: &'static str },
    #[error("对方拒绝连接（原因 {}）", .0.reason.0)]
    Rejected(Reject),
    #[error("无法初始化 TLS：{0}")]
    TlsConfig(#[from] rustls::Error),
    #[error("对方不是 ScreenFerry（TLS 协议标识不符）")]
    WrongProtocol,
    #[error("对方没有出示证书")]
    NoPeerCertificate,
    #[error("局域网发现失败：{0}")]
    Discovery(String),
    #[error("{0}")]
    Http(String),
    #[error("内容比预期的大")]
    TooLarge,
    #[error("已取消")]
    Cancelled,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
