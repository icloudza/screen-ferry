//! 全项目共用的领域类型。
//!
//! 设备与屏幕 ID、HID 键码、输入事件、几何、配置结构（含每台设备的单独设置）、文件传输编号、浮层的绘制描述，
//! 以及运行日志里敏感内容的标记（[`mark`]）。
//!
//! 层级：L0。不依赖任何其他 ferry crate。

#![forbid(unsafe_code)]

mod clipboard;
mod config;
mod device;
mod display;
mod geometry;
mod input;
mod input_proof;
mod key;
pub mod mark;
mod pointer;
mod scene;
mod transfer;

pub use clipboard::{ClipboardContent, ClipboardFormat};
pub use config::{Config, Lang, Language, Placement, Role, Theme, uses_command_key};
pub use device::{
    DeviceSettings, KeyMapping, ModifierMap, ModifierPreset, ModifierTarget, ScrollSpeed,
};
pub use display::{Display, DisplayId};
pub use geometry::{Point, Rect, Side};
pub use input::InputEvent;
pub use input_proof::InputProof;
pub use key::{KeyAction, KeyCode, LockKeys};
pub use pointer::{MouseButton, WHEEL_NOTCH};
pub use scene::{Align, Frame, Hit, Rgba, Scene, Shape, Weight};
pub use transfer::{ParseTransferIdError, TransferId};
