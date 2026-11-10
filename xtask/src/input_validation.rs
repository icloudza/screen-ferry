//! 构建隔离 Windows 实机验收所需的两个 CLI 程序，不安装或启动服务。

use std::fs;
use std::path::Path;
use std::process::Command;

use ring::digest::{SHA256, digest};
use serde_json::json;

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let target = match args {
        [] if std::env::consts::OS == "windows" => "x86_64-pc-windows-msvc",
        [flag, target] if flag == "--target" && matches!(target.as_str(), "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu") => target,
        _ => return Err("用法：cargo xtask input-validation [--target x86_64-pc-windows-msvc|x86_64-pc-windows-gnu]；仅用于隔离测试".into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("没有仓库根目录")?;
    let status = Command::new("cargo")
        .current_dir(root)
        .args([
            "build",
            "--release",
            "--locked",
            "-p",
            "ferry-service",
            "-p",
            "ferry-daemon",
            "--target",
            target,
        ])
        .status()
        .map_err(|e| format!("无法构建验收程序：{e}"))?;
    if !status.success() {
        return Err("验收程序构建失败".into());
    }

    let output = root.join("target/input-validation/windows");
    if output.exists() {
        fs::remove_dir_all(&output).map_err(|e| e.to_string())?;
    }
    let programs = output.join("ScreenFerry");
    fs::create_dir_all(&programs).map_err(|e| e.to_string())?;
    let mut hashes = serde_json::Map::new();
    for name in ["screenferry.exe", "screenferry-service.exe"] {
        let bytes = fs::read(root.join("target").join(target).join("release").join(name))
            .map_err(|e| format!("{name}：{e}"))?;
        let sha256: String = digest(&SHA256, &bytes)
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        fs::write(programs.join(name), &bytes).map_err(|e| e.to_string())?;
        hashes.insert(
            name.into(),
            json!({ "bytes": bytes.len(), "sha256": sha256 }),
        );
    }
    fs::write(
        output.join("README.md"),
        include_str!("../../docs/windows-secure-input-validation.md"),
    )
    .map_err(|e| e.to_string())?;
    let metadata = json!({
        "purpose": "isolated Windows CLI acceptance testing; not a release installer",
        "target": target,
        "profile": "release",
        "source_version": env!("CARGO_PKG_VERSION"),
        "protocol": ferry_proto::WIRE_FORMAT,
        "files": hashes,
    });
    fs::write(
        output.join("build.json"),
        serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let archive = root.join("target/input-validation/ScreenFerry-secure-input-windows-x64.zip");
    let mut pack = match std::env::consts::OS {
        "macos" => {
            let mut command = Command::new("ditto");
            command
                .args(["-c", "-k", "--keepParent"])
                .arg(&output)
                .arg(&archive);
            Some(command)
        }
        "windows" => {
            let mut command = Command::new("powershell.exe");
            // 固定脚本文本；路径通过环境变量传入，不拼接成 PowerShell 代码。
            command.args(["-NoProfile", "-NonInteractive", "-Command", "Compress-Archive -LiteralPath $env:SCREENFERRY_VALIDATION_SOURCE -DestinationPath $env:SCREENFERRY_VALIDATION_ZIP -Force"])
                .env("SCREENFERRY_VALIDATION_SOURCE", &output).env("SCREENFERRY_VALIDATION_ZIP", &archive);
            Some(command)
        }
        _ => None,
    };
    if let Some(command) = &mut pack {
        if archive.exists() {
            fs::remove_file(&archive).map_err(|e| e.to_string())?;
        }
        let status = command
            .status()
            .map_err(|e| format!("无法压缩验收包：{e}"))?;
        if !status.success() {
            return Err("压缩验收包失败，已生成的程序仍在验收目录中".into());
        }
        println!("压缩包：{}", archive.display());
    }
    println!(
        "验收包：{}（仅限隔离 Windows 测试机；服务未安装、未启动）",
        output.display()
    );
    Ok(())
}
