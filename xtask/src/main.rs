//! 仓库自动化任务。用法：`cargo xtask <命令>`。

#![forbid(unsafe_code)]

mod bundle;
mod deps;
mod icons;
mod input_validation;
mod layout;
mod release;
mod workspace;

use std::process::ExitCode;

const USAGE: &str = "\
用法：cargo xtask <命令>

命令：
  lint    检查仓库是否符合 AGENTS.md 中的结构规则与依赖规则
  bundle  打包：macOS 上生成 target/bundle/macos/ScreenFerry.app（后台 + 设置界面）
  bundle [--universal] [--notarize]
          macOS 上可以打通用版、交给 Apple 公证；也会生成更新包 ScreenFerry-<版本>-macos.zip
  icons   从 assets/icons/ 的 SVG 导出托盘图标和两个系统的应用图标（需要 rsvg-convert）
  input-validation [--target <Windows目标>]  构建安全输入 CLI 验收包，不安装或启动
  release <版本>   改版本号、整理更新日志，提交并打 tag（推送后 CI 构建发布草稿）
  manifest …       CI 用：按目录里的更新包写 update.json
  sign <文件>      CI 用：用发布私钥签名，写出 <文件>.minisig
  update-keygen    生成发布密钥，公钥写进 crates/ferry-update/src/keys.rs（只需一次）";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("lint") => lint(),
        Some("bundle") => match bundle::run(&std::env::args().skip(2).collect::<Vec<_>>()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("打包失败：{e}");
                ExitCode::FAILURE
            }
        },
        Some("icons") => match icons::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("导出图标失败：{e}");
                ExitCode::FAILURE
            }
        },
        Some("input-validation") => {
            match input_validation::run(&std::env::args().skip(2).collect::<Vec<_>>()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("验收包构建失败：{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some(cmd @ ("release" | "manifest" | "sign" | "update-keygen")) => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            let result = match cmd {
                "release" => release::release(&args),
                "manifest" => release::manifest(&args),
                "sign" => release::sign(&args),
                _ => release::keygen(&args),
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{cmd} 失败：{e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn lint() -> ExitCode {
    let ws = match workspace::load() {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("读取 workspace 失败：{e}");
            return ExitCode::FAILURE;
        }
    };

    let mut violations = Vec::new();
    layout::check(&ws, &mut violations);
    deps::check(&ws, &mut violations);

    if violations.is_empty() {
        println!("lint 通过：{} 个 crate 均符合 AGENTS.md", ws.crates.len());
        return ExitCode::SUCCESS;
    }
    for v in &violations {
        eprintln!("✗ {v}");
    }
    eprintln!("\n共 {} 处违反 AGENTS.md", violations.len());
    ExitCode::FAILURE
}
