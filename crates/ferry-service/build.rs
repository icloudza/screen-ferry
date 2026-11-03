//! 给 Windows 服务嵌入与后台相同的版本和清单；其他平台不编译资源。

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir")).join("../..");
    let packaging = root.join("packaging/windows");
    for path in [
        packaging.join("screenferry-service.rc"),
        packaging.join("app.manifest"),
        root.join("assets/icons/app-icon-windows.ico"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let version = env::var("CARGO_PKG_VERSION").expect("version");
    let parts: Vec<_> = version
        .split('.')
        .map(|p| p.parse::<u16>().unwrap_or(0))
        .collect();
    let out = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    fs::write(out.join("version.h"), format!("#define VERSION_MAJOR {}\n#define VERSION_MINOR {}\n#define VERSION_PATCH {}\n#define VERSION_TEXT \"{version}\"\n", parts[0], parts[1], parts[2])).expect("version header");
    embed_resource::compile(
        packaging.join("screenferry-service.rc"),
        embed_resource::ParamsIncludeDirs([&packaging]),
    )
    .manifest_required()
    .expect("Windows service resources");
}
