//! Windows 安全桌面输入服务的 L4 入口。只负责参数和进程组织；Windows API 在平台实现中。

#![forbid(unsafe_code)]

mod app;
mod cli;

pub use app::{init_output, run};
pub use cli::{Mode, parse};
