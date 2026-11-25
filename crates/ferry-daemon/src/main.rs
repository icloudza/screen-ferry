//! screenferry：后台常驻进程，负责键鼠共享、剪贴板与文件同步、托盘。
//!
//! 层级：L4。按目标平台选择 `ferry-platform-*` 实现并注入 `ferry-engine`。

#![forbid(unsafe_code)]
// Windows：不带控制台，双击、登录时启动不会闪出黑色窗口。命令行用法在 `app::init_output` 里接回终端。
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod cli;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod host;
mod logging;
mod pairing;
mod preview;
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod tray;

use std::process::ExitCode;

fn main() -> ExitCode {
    let parsed = cli::parse(std::env::args().skip(1));
    app::init_output(&parsed);
    match parsed {
        Ok(cli::Parsed::Run(options)) => app::run(options),
        Ok(cli::Parsed::Help) => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}\n\n{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}
