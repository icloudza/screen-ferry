//! screenferry-service：可选的 Windows 安全桌面输入服务。

#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    let mode = ferry_service::parse(std::env::args().skip(1));
    ferry_service::init_output();
    match mode.and_then(ferry_service::run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
