//! Windows 特权输入：会话服务、受限命名管道及按桌面运行的代理。

mod agent;
mod client;
mod controller;
mod pipe;
mod policy;
mod process;
mod security;
mod util;

pub use agent::run_agent;
pub use client::WinInject;
pub use controller::{install, prepare, remove, run, stop};
pub use policy::{authorize, revoke};

const NAME: &str = "ScreenFerryInput";
const SERVICE_EXE: &str = "screenferry-service.exe";
