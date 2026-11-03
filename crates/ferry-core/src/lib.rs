//! 核心逻辑：屏幕布局、边缘切换、多台设备的布局与切换、会话状态机、按键状态跟踪、收到的文件名规则、边缘投递胶囊、
//! 剪贴板格式转换、挪动光标后位移的修正，每台设备的按键映射和滚轮换算，以及导出诊断文件时的代号替换。
//!
//! 层级：L1。纯逻辑，无 IO、无异步。

#![forbid(unsafe_code)]

pub mod capsule;
pub mod clip_format;
mod edge_switch;
pub mod file_name;
mod input_lease;
mod key_mapper;
mod layout;
mod neighbors;
mod pressed;
mod redact;
mod warp_jump;
mod wheel_scale;

pub use edge_switch::{EdgeSwitch, Transition};
pub use input_lease::{INPUT_LEASE, InputLease, MAX_INPUT_REQUEST_MS};
pub use key_mapper::KeyMapper;
pub use layout::{Side, bounds};
pub use neighbors::{Neighbors, Place};
pub use pressed::Pressed;
pub use redact::Redactor;
pub use warp_jump::WarpJump;
pub use wheel_scale::WheelScale;
