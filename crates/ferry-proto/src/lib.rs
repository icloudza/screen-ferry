//! 线协议：帧格式、消息定义、版本与能力协商。
//!
//! 本 crate 只做编解码，不做 IO：[`Codec`] 把 [`Message`] 写进字节缓冲区，
//! 或从字节缓冲区读出。连接读写、握手顺序、心跳超时由 ferry-net 和 ferry-engine 负责。
//!
//! 线上格式的完整说明见 `docs/protocol.md`。
//!
//! 层级：L1。纯逻辑，无 IO、无异步。

#![forbid(unsafe_code)]

pub mod input_agent;
pub mod limits;

mod clipboard;
mod codec;
mod edge;
mod error;
mod file;
mod handshake;
mod message;
mod screen;
mod secure_input;
mod transfer;
mod wire;

pub use clipboard::{ClipboardItem, ClipboardOffer, ClipboardRequest};
pub use codec::{Codec, HEADER_LEN};
pub use edge::{DropHint, HintState};
pub use error::{DecodeError, EncodeError};
pub use file::{EntryKind, FileCancel, FileEnd, FileEntry, FileList, FileOffer, FileResume};
pub use handshake::{
    Bye, ByeReason, Capabilities, Channel, Hello, Reject, RejectReason, WIRE_FORMAT, Welcome,
};
pub use message::Message;
pub use screen::Enter;
pub use secure_input::SecureAction;
pub use transfer::{Cancel, CancelReason, Chunk};
