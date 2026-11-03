//! `cargo xtask bundle`：把后台与设置界面打成一个应用。在哪个系统上运行就打哪个系统的包。
//!
//! macOS 产物是 `target/bundle/macos/ScreenFerry.app`：
//!
//! ```text
//! ScreenFerry.app/Contents/
//! ├─ Info.plist                      packaging/macos/Info.plist，填入版本号
//! ├─ MacOS/screenferry               后台（主程序，不在程序坞里显示）
//! ├─ Resources/AppIcon.icns          取自设置界面
//! └─ Helpers/ScreenFerry.app      Flutter 设置界面（和外层同名：程序坞里显示的就是软件名）
//! ```
//!
//! 签名：环境变量 `SCREENFERRY_SIGN_IDENTITY` 指定的证书优先，其次钥匙串里第一张
//! “Developer ID Application”证书，都没有时用临时签名（ad-hoc）。用固定的开发者证书签名后，
//! 系统按“团队 + bundle id”认这个应用，重新打包也不会丢掉已经授予的辅助功能权限；
//! 临时签名每次打包都像一个新应用。签名开启 hardened runtime。
//!
//! 最后把应用打成 `ScreenFerry-<版本>-macos.zip`，它就是自动更新下载的更新包（ADR 0014）。选项：
//!
//! - `--universal`：后台同时编译 arm64 和 x86_64 两份再合成一个（设置界面本来就是通用的）。
//!   需要先 `rustup target add aarch64-apple-darwin x86_64-apple-darwin`。
//! - `--notarize`：交给 Apple 公证并装订票据。需要环境变量 `SCREENFERRY_NOTARY_KEY`
//!   （App Store Connect API 密钥 .p8 文件的路径）、`SCREENFERRY_NOTARY_KEY_ID`、`SCREENFERRY_NOTARY_ISSUER`。
//! - `--dmg`：另外做一个安装盘 `ScreenFerry-<版本>-macos-<架构>.dmg`，第一次安装用：打开后把应用拖进
//!   “应用程序”。窗口背景是 `assets/dmg/background.png`（和 @2x），布局由 dmgbuild 写进盘里，
//!   不用打开访达；需要 `dmgbuild`（`pip install dmgbuild`，或用环境变量 `SCREENFERRY_DMGBUILD` 指定）。
//!   有开发者证书时给盘签名；同时开着 `--notarize` 时盘也交公证、装订票据。
//!
//! 更新包和安装盘的名字里带架构：通用版是 `macos-universal`，否则是本机的 `macos-arm64` 或 `macos-x86_64`。
//!
//! Windows 先组装 `target/bundle/windows/ScreenFerry/` 目录，再用 Inno Setup 打成安装包
//! `target/bundle/windows/ScreenFerry-<版本>-windows-x64-setup.exe`（`packaging/windows/screenferry.iss`，ADR 0011）：
//!
//! ```text
//! ScreenFerry\
//! ├─ screenferry.exe                后台，C 运行库静态链接；图标、版本信息、程序清单见 packaging/windows/
//! ├─ screenferry-service.exe        可选的 Windows 安全桌面输入服务
//! └─ settings\                      Flutter 设置界面：screenferry-settings.exe、flutter_windows.dll、data\，
//!                                    以及它需要的 Visual C++ 运行库（msvcp140.dll 等三个）
//! ```
//!
//! 需要 Visual Studio（C++ 工具，附带运行库）和 Inno Setup 7.1 或更高版本（自带简体中文）；
//! ISCC.exe 不在默认位置时用环境变量 `ISCC` 指定。
//! `--experimental-secure-input` 仅供隔离 Windows 虚拟机验证，开放尚未完成安全验收的 UAC 服务选项。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T = ()> = std::result::Result<T, String>;

/// 设置界面在应用里的位置。和外层同名，程序坞、切换器里显示的都是软件名。
const HELPER: &str = "Helpers/ScreenFerry.app";
/// 系统登记应用的工具，不在 PATH 里。
const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";

/// 打包选项，见本文件开头。
#[derive(Default)]
struct Options {
    experimental_secure_input: bool,
    universal: bool,
    notarize: bool,
    dmg: bool,
}

pub(crate) fn run(args: &[String]) -> Result {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("找不到仓库根目录")?
        .to_path_buf();
    let mut options = Options::default();
    for arg in args {
        match arg.as_str() {
            "--experimental-secure-input" => options.experimental_secure_input = true,
            "--universal" => options.universal = true,
            "--notarize" => options.notarize = true,
            "--dmg" => options.dmg = true,
            other => return Err(format!("不认识的选项 {other}")),
        }
    }
    match std::env::consts::OS {
        "macos" if options.experimental_secure_input => {
            Err("实验安全输入构建仅支持 Windows".into())
        }
        "macos" => macos(&root, &options),
        "windows" => windows(&root, &options),
        other => Err(format!("还不支持在 {other} 上打包")),
    }
}

/// 设置界面（Flutter）依赖的 Visual C++ 运行库，放在它旁边，没装过运行库的电脑也能打开。
const VC_RUNTIME: [&str; 3] = ["msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll"];

fn windows(root: &Path, options: &Options) -> Result {
    let version = workspace_version(root)?;
    // 先找齐工具，免得编译完才发现缺东西。
    let iscc = find_iscc()?;
    let runtime = vc_runtime_dir()?;

    step("编译后台（release）");
    let mut build = vec![
        "build",
        "--release",
        "-p",
        "ferry-daemon",
        "-p",
        "ferry-service",
    ];
    if options.experimental_secure_input {
        build.extend(["--features", "ferry-service/experimental-secure-input"]);
    }
    cargo(root, &build)?;

    step("编译设置界面（flutter build windows --release）");
    let app_dir = root.join("app");
    flutter(&app_dir, &["build", "windows", "--release"])?;
    let built = app_dir.join("build/windows/x64/runner/Release");

    step("组装 ScreenFerry 目录");
    let out = root.join("target/bundle/windows/ScreenFerry");
    if out.exists() {
        fs::remove_dir_all(&out).map_err(|e| io(&out, e))?;
    }
    fs::create_dir_all(&out).map_err(|e| io(&out, e))?;
    copy(
        &root.join("target/release/screenferry.exe"),
        &out.join("screenferry.exe"),
    )?;
    copy_dir(&built, &out.join("settings"))?;
    copy(
        &root.join("target/release/screenferry-service.exe"),
        &out.join("screenferry-service.exe"),
    )?;
    for dll in VC_RUNTIME {
        copy(&runtime.join(dll), &out.join("settings").join(dll))?;
    }

    step("打安装包（Inno Setup）");
    let installers = root.join("target/bundle/windows");
    let setup = installers.join(format!("ScreenFerry-{version}-windows-x64-setup.exe"));
    if setup.exists() {
        fs::remove_file(&setup).map_err(|e| io(&setup, e))?;
    }
    let mut command = Command::new(&iscc);
    if options.experimental_secure_input {
        command.arg("/DSecureInputExperimental=1");
    }
    let status = command
        .arg("--quiet-progress")
        .arg(format!("/DAppVersion={version}"))
        .arg(format!("/DSourceDir={}", out.display()))
        .arg(format!("--output-dir={}", installers.display()))
        .arg(root.join("packaging/windows/screenferry.iss"))
        .status()
        .map_err(|e| format!("无法运行 {}：{e}", iscc.display()))?;
    if !status.success() || !setup.exists() {
        return Err("Inno Setup 没有打出安装包".into());
    }

    println!("\n✓ {}", setup.display());
    println!(
        "  双击安装：装进 Program Files，开始菜单里有 ScreenFerry，可以在“设置 → 应用”里卸载。"
    );
    println!(
        "  不安装直接试：双击 {}",
        out.join("screenferry.exe").display()
    );
    Ok(())
}

/// 找 Inno Setup 的命令行编译器：环境变量 `ISCC`，或者默认的安装位置。
/// 需要 7.1 或更高版本：从这一版起自带简体中文。
fn find_iscc() -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("ISCC")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for (var, sub) in [
        ("ProgramFiles", ""),
        ("ProgramFiles(x86)", ""),
        ("LOCALAPPDATA", "Programs"),
    ] {
        if let Some(dir) = std::env::var_os(var) {
            candidates.push(
                PathBuf::from(dir)
                    .join(sub)
                    .join("Inno Setup 7")
                    .join("ISCC.exe"),
            );
        }
    }
    let iscc = candidates.into_iter().find(|p| p.is_file()).ok_or(
        "找不到 Inno Setup 7 的 ISCC.exe：从 https://jrsoftware.org/isdl.php 安装 7.1 或更高版本，\
         或者用环境变量 ISCC 指定",
    )?;
    let chinese = iscc
        .with_file_name("Languages")
        .join("ChineseSimplified.isl");
    if !chinese.is_file() {
        return Err(format!(
            "{} 太旧：需要 Inno Setup 7.1 或更高版本（自带简体中文）",
            iscc.display()
        ));
    }
    Ok(iscc)
}

/// Visual C++ 运行库所在的目录（`…\VC\Redist\MSVC\<版本>\x64\Microsoft.VC14x.CRT`）。
/// 开发者命令行里有 `VCToolsRedistDir`；否则用 vswhere 找装了 C++ 工具的 Visual Studio，取最新的一版。
fn vc_runtime_dir() -> Result<PathBuf> {
    let redist = match std::env::var_os("VCToolsRedistDir") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let program_files =
                std::env::var_os("ProgramFiles(x86)").ok_or("找不到 Program Files (x86)")?;
            let vswhere =
                PathBuf::from(program_files).join(r"Microsoft Visual Studio\Installer\vswhere.exe");
            let output = Command::new(&vswhere)
                .args(["-latest", "-products", "*", "-requires"])
                .arg("Microsoft.VisualStudio.Component.VC.Tools.x86.x64")
                .args(["-property", "installationPath"])
                .output()
                .map_err(|e| {
                    format!("无法运行 vswhere（需要装有 C++ 工具的 Visual Studio）：{e}")
                })?;
            let install = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if install.is_empty() {
                return Err("找不到装有 C++ 工具的 Visual Studio".into());
            }
            let msvc = PathBuf::from(install).join(r"VC\Redist\MSVC");
            subdirs(&msvc)?
                .into_iter()
                .filter_map(|dir| Some((version_key(&dir)?, dir)))
                .max()
                .map(|(_, dir)| dir)
                .ok_or_else(|| format!("{} 里没有运行库", msvc.display()))?
        }
    };
    let x64 = redist.join("x64");
    subdirs(&x64)?
        .into_iter()
        .filter(|dir| {
            dir.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("Microsoft.VC") && n.ends_with(".CRT"))
        })
        .max()
        .filter(|dir| VC_RUNTIME.iter().all(|dll| dir.join(dll).is_file()))
        .ok_or_else(|| format!("{} 里没有完整的 Visual C++ 运行库", x64.display()))
}

/// 目录下的子目录。
fn subdirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io(dir, e))? {
        let path = entry.map_err(|e| io(dir, e))?.path();
        if path.is_dir() {
            found.push(path);
        }
    }
    Ok(found)
}

/// 形如 `14.44.35112` 的目录名按数字比较；不是版本号的（例如 `v143`）返回 `None`。
fn version_key(dir: &Path) -> Option<Vec<u32>> {
    dir.file_name()?
        .to_str()?
        .split('.')
        .map(|part| part.parse().ok())
        .collect()
}

fn macos(root: &Path, options: &Options) -> Result {
    let root = root.to_path_buf();
    let version = workspace_version(&root)?;
    let notary = if options.notarize {
        Some(Notary::from_env()?)
    } else {
        None
    };
    let dmgbuild = if options.dmg {
        Some(find_dmgbuild()?)
    } else {
        None
    };
    let platform = if options.universal {
        "macos-universal"
    } else if std::env::consts::ARCH == "aarch64" {
        "macos-arm64"
    } else {
        "macos-x86_64"
    };

    let daemon = if options.universal {
        universal_daemon(&root)?
    } else {
        step("编译后台（release）");
        cargo(&root, &["build", "--release", "-p", "ferry-daemon"])?;
        root.join("target/release/screenferry")
    };

    step("编译设置界面（flutter build macos --release）");
    let app_dir = root.join("app");
    flutter(&app_dir, &["build", "macos", "--release"])?;
    let settings = app_dir.join("build/macos/Build/Products/Release/ScreenFerry.app");

    step("组装 ScreenFerry.app");
    let bundle = root.join("target/bundle/macos/ScreenFerry.app");
    if bundle.exists() {
        fs::remove_dir_all(&bundle).map_err(|e| io(&bundle, e))?;
    }
    let contents = bundle.join("Contents");
    for dir in ["MacOS", "Resources", "Helpers"] {
        fs::create_dir_all(contents.join(dir)).map_err(|e| io(&contents, e))?;
    }

    let plist = fs::read_to_string(root.join("packaging/macos/Info.plist"))
        .map_err(|e| io(&root.join("packaging/macos/Info.plist"), e))?
        .replace("{{VERSION}}", &version);
    write(&contents.join("Info.plist"), plist.as_bytes())?;

    copy(&daemon, &contents.join("MacOS/screenferry"))?;
    copy(
        &settings.join("Contents/Resources/AppIcon.icns"),
        &contents.join("Resources/AppIcon.icns"),
    )?;
    // ditto 保留 Flutter 框架里的符号链接和已有的签名。
    run_in(
        &root,
        "ditto",
        &[
            settings.to_str().ok_or("路径不是 UTF-8")?,
            contents.join(HELPER).to_str().ok_or("路径不是 UTF-8")?,
        ],
    )?;

    let identity = signing_identity();
    step(&format!(
        "签名（{}）",
        if identity == "-" {
            "临时签名，仅限本机"
        } else {
            identity.as_str()
        }
    ));
    let runtime: &[&str] = if identity == "-" {
        &[]
    } else {
        &["--options", "runtime"]
    };
    // Flutter 产物里的 App.framework 签名后又被改过，严格校验通不过；先整体重签内层的设置界面。
    let helper = contents.join(HELPER);
    let mut args = vec!["--force", "--deep", "--sign", identity.as_str()];
    args.extend_from_slice(runtime);
    args.push(helper.to_str().ok_or("路径不是 UTF-8")?);
    run_in(&root, "codesign", &args)?;

    let entitlements = root.join("packaging/macos/entitlements.plist");
    let mut args = vec!["--force", "--sign", identity.as_str()];
    args.extend_from_slice(runtime);
    args.extend_from_slice(&[
        "--entitlements",
        entitlements.to_str().ok_or("路径不是 UTF-8")?,
        bundle.to_str().ok_or("路径不是 UTF-8")?,
    ]);
    run_in(&root, "codesign", &args)?;

    run_in(
        &root,
        "codesign",
        &[
            "--verify",
            "--deep",
            "--strict",
            bundle.to_str().ok_or("路径不是 UTF-8")?,
        ],
    )?;

    if let Some(notary) = &notary {
        if identity == "-" {
            return Err("公证需要 Developer ID 证书签名，钥匙串里没有找到".into());
        }
        notarize(&root, &bundle, notary)?;
    }

    refresh_icons(&root, &[&bundle, &helper]);

    step("打成更新包");
    let zip = root.join(format!(
        "target/bundle/macos/ScreenFerry-{version}-{platform}.zip"
    ));
    if zip.exists() {
        fs::remove_file(&zip).map_err(|e| io(&zip, e))?;
    }
    run_in(
        &root,
        "ditto",
        &[
            "-c",
            "-k",
            "--keepParent",
            bundle.to_str().ok_or("路径不是 UTF-8")?,
            zip.to_str().ok_or("路径不是 UTF-8")?,
        ],
    )?;

    let dmg = match &dmgbuild {
        Some(tool) => {
            let dmg = root.join(format!(
                "target/bundle/macos/ScreenFerry-{version}-{platform}.dmg"
            ));
            make_dmg(&root, tool, &bundle, &dmg)?;
            if identity != "-" {
                step("给安装盘签名");
                let path = dmg.to_str().ok_or("路径不是 UTF-8")?;
                run_in(
                    &root,
                    "codesign",
                    &["--force", "--timestamp", "--sign", &identity, path],
                )?;
                if let Some(notary) = &notary {
                    submit(&root, &dmg, notary)?;
                    step("给安装盘装订公证票据");
                    run_in(&root, "xcrun", &["stapler", "staple", path])?;
                    run_in(
                        &root,
                        "spctl",
                        &[
                            "--assess",
                            "--type",
                            "open",
                            "--context",
                            "context:primary-signature",
                            path,
                        ],
                    )?;
                }
            }
            Some(dmg)
        }
        None => None,
    };

    println!("\n✓ {}", bundle.display());
    println!("  更新包：{}", zip.display());
    if let Some(dmg) = &dmg {
        println!("  安装盘：{}", dmg.display());
    }
    println!("  打开：open \"{}\"", bundle.display());
    println!("  安装：拖到“应用程序”文件夹。首次运行需要在系统设置里允许它控制键盘和鼠标。");
    if identity == "-" {
        println!("        临时签名的应用每次重新打包后，可能要在“辅助功能”里关掉再打开一次。");
    }
    Ok(())
}

/// 后台的通用版：分别编译 arm64 和 x86_64，再用 lipo 合成一个。
fn universal_daemon(root: &Path) -> Result<PathBuf> {
    const TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
    let installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("无法运行 rustup：{e}"))?;
    let installed = String::from_utf8_lossy(&installed.stdout);
    if let Some(missing) = TARGETS
        .iter()
        .find(|t| !installed.lines().any(|l| l.trim() == **t))
    {
        return Err(format!(
            "缺少编译目标 {missing}：先运行 rustup target add {}",
            TARGETS.join(" ")
        ));
    }
    let mut parts = Vec::new();
    for target in TARGETS {
        step(&format!("编译后台（release，{target}）"));
        cargo(
            root,
            &[
                "build",
                "--release",
                "-p",
                "ferry-daemon",
                "--target",
                target,
            ],
        )?;
        parts.push(root.join(format!("target/{target}/release/screenferry")));
    }
    step("合成通用版（lipo）");
    let out = root.join("target/release/screenferry-universal");
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    }
    let mut args = vec!["-create", "-output", out.to_str().ok_or("路径不是 UTF-8")?];
    for part in &parts {
        args.push(part.to_str().ok_or("路径不是 UTF-8")?);
    }
    run_in(root, "lipo", &args)?;
    Ok(out)
}

/// 公证用的 App Store Connect API 密钥。
struct Notary {
    key: String,
    key_id: String,
    issuer: String,
}

impl Notary {
    fn from_env() -> Result<Self> {
        let var = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| format!("公证需要环境变量 {name}"))
        };
        Ok(Self {
            key: var("SCREENFERRY_NOTARY_KEY")?,
            key_id: var("SCREENFERRY_NOTARY_KEY_ID")?,
            issuer: var("SCREENFERRY_NOTARY_ISSUER")?,
        })
    }
}

/// 交给 Apple 公证，等结果，通过后把票据装订进应用，再用 Gatekeeper 检查一遍。
fn notarize(root: &Path, bundle: &Path, notary: &Notary) -> Result {
    let upload = root.join("target/bundle/macos/ScreenFerry-notarize.zip");
    let _ = fs::remove_file(&upload);
    let bundle_path = bundle.to_str().ok_or("路径不是 UTF-8")?;
    let upload_path = upload.to_str().ok_or("路径不是 UTF-8")?;
    run_in(
        root,
        "ditto",
        &["-c", "-k", "--keepParent", bundle_path, upload_path],
    )?;
    let submitted = submit(root, &upload, notary);
    let _ = fs::remove_file(&upload);
    submitted?;
    step("装订公证票据");
    run_in(root, "xcrun", &["stapler", "staple", bundle_path])?;
    run_in(
        root,
        "spctl",
        &["--assess", "--type", "execute", bundle_path],
    )
}

/// 交给 Apple 公证（zip、dmg 都行），等到有结果。
fn submit(root: &Path, file: &Path, notary: &Notary) -> Result {
    step(&format!(
        "公证 {}（xcrun notarytool，通常要几分钟）",
        file.file_name().and_then(|n| n.to_str()).unwrap_or("")
    ));
    let output = Command::new("xcrun")
        .args(["notarytool", "submit"])
        .arg(file)
        .arg("--wait")
        .args([
            "--key",
            &notary.key,
            "--key-id",
            &notary.key_id,
            "--issuer",
            &notary.issuer,
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("无法运行 notarytool：{e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    print!("{text}");
    if !output.status.success() || !text.contains("status: Accepted") {
        return Err(format!(
            "公证没有通过：{}（可以用 xcrun notarytool log <提交号> 查看原因）",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// 安装盘窗口：内容 660 × 400 加 28 高的标题栏；图标 128、文件名 13，两个图标的位置和
/// assets/dmg/background.svg 对齐。
const DMG_WINDOW: (u32, u32) = (660, 428);
const DMG_APP_AT: (u32, u32) = (165, 210);
const DMG_APPLICATIONS_AT: (u32, u32) = (495, 210);

/// dmgbuild：环境变量 `SCREENFERRY_DMGBUILD` 指定的，否则 PATH 里的。
fn find_dmgbuild() -> Result<String> {
    let tool = std::env::var("SCREENFERRY_DMGBUILD")
        .ok()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "dmgbuild".to_owned());
    let found = Command::new(&tool)
        .arg("--help")
        .output()
        .is_ok_and(|o| o.status.success());
    if found {
        Ok(tool)
    } else {
        Err("找不到 dmgbuild：pip install dmgbuild，或用环境变量 SCREENFERRY_DMGBUILD 指定".into())
    }
}

/// 做安装盘：应用、指向“应用程序”的替身、背景图、卷图标，窗口布局写进盘里。
fn make_dmg(root: &Path, tool: &str, bundle: &Path, dmg: &Path) -> Result {
    step("做安装盘（dmgbuild）");
    if dmg.exists() {
        fs::remove_file(dmg).map_err(|e| io(dmg, e))?;
    }
    let quote = |path: &Path| -> Result<String> {
        let text = path.to_str().ok_or("路径不是 UTF-8")?;
        serde_json::to_string(text).map_err(|e| e.to_string())
    };
    let name = bundle
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("应用的名字不是 UTF-8")?;
    let (width, height) = DMG_WINDOW;
    let settings = format!(
        "# 由 cargo xtask bundle --dmg 生成，每次打包都会重写。\n\
         format = \"ULFO\"\n\
         filesystem = \"HFS+\"\n\
         files = [{app}]\n\
         symlinks = {{\"Applications\": \"/Applications\"}}\n\
         icon = {icon}\n\
         background = {background}\n\
         window_rect = ((200, 120), ({width}, {height}))\n\
         icon_size = 128\n\
         text_size = 13\n\
         icon_locations = {{\"{name}\": {app_at:?}, \"Applications\": {apps_at:?}}}\n\
         hide_extensions = [\"{name}\"]\n\
         show_status_bar = False\n\
         show_toolbar = False\n\
         show_pathbar = False\n\
         show_sidebar = False\n",
        app = quote(bundle)?,
        icon = quote(&bundle.join("Contents/Resources/AppIcon.icns"))?,
        background = quote(&root.join("assets/dmg/background.png"))?,
        app_at = DMG_APP_AT,
        apps_at = DMG_APPLICATIONS_AT,
    );
    let path = root.join("target/bundle/macos/dmg-settings.py");
    write(&path, settings.as_bytes())?;
    run_in(
        root,
        tool,
        &[
            "-s",
            path.to_str().ok_or("路径不是 UTF-8")?,
            "ScreenFerry",
            dmg.to_str().ok_or("路径不是 UTF-8")?,
        ],
    )
}

/// 访达和程序坞按路径缓存应用图标，原地重新打包后还显示旧图标。碰一下修改时间、重新登记，
/// 让它们重新读。做不到不影响打包。
fn refresh_icons(root: &Path, apps: &[&Path]) {
    for app in apps {
        let Some(path) = app.to_str() else { continue };
        if run_in(root, "touch", &[path]).is_err()
            || run_in(root, LSREGISTER, &["-f", path]).is_err()
        {
            eprintln!("提示：没能刷新 {path} 的图标缓存，可能要重新登录才显示新图标");
        }
    }
}

/// 选签名证书：环境变量指定的优先，其次钥匙串里第一张 Developer ID Application 证书，
/// 都没有时用临时签名 `-`。
fn signing_identity() -> String {
    if let Ok(identity) = std::env::var("SCREENFERRY_SIGN_IDENTITY")
        && !identity.trim().is_empty()
    {
        return identity;
    }
    let output = Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output();
    let Ok(output) = output else {
        return "-".into();
    };
    // 每行形如：  1) <40 位哈希> "Developer ID Application: 名字 (团队)"
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains("\"Developer ID Application:"))
        .find_map(|line| line.split('"').nth(1).map(str::to_owned))
        .unwrap_or_else(|| "-".into())
}

fn workspace_version(root: &Path) -> Result<String> {
    let text = fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?;
    let manifest: toml::Table = text.parse().map_err(|e| format!("Cargo.toml：{e}"))?;
    manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "Cargo.toml 中没有 [workspace.package] version".into())
}

fn step(what: &str) {
    println!("==> {what}");
}

fn cargo(root: &Path, args: &[&str]) -> Result {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.args(args).current_dir(root);
    // 诊断文件里写上是哪次提交打的包。
    if let Some(commit) = commit(root) {
        command.env("SCREENFERRY_COMMIT", commit);
    }
    let status = command
        .status()
        .map_err(|e| format!("无法运行 cargo：{e}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("cargo {} 失败", args.join(" ")))
}

fn run_in(dir: &Path, program: &str, args: &[&str]) -> Result {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("无法运行 {program}：{e}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{program} {} 失败", args.join(" ")))
}

/// Windows 上的 flutter 是批处理文件，要经 cmd 运行。
fn flutter(dir: &Path, args: &[&str]) -> Result {
    if std::env::consts::OS == "windows" {
        let mut all = vec!["/C", "flutter"];
        all.extend_from_slice(args);
        run_in(dir, "cmd", &all)
    } else {
        run_in(dir, "flutter", args)
    }
}

/// 递归复制目录。
fn copy_dir(from: &Path, to: &Path) -> Result {
    fs::create_dir_all(to).map_err(|e| io(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| io(from, e))? {
        let entry = entry.map_err(|e| io(from, e))?;
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            copy(&path, &target)?;
        }
    }
    Ok(())
}

fn copy(from: &Path, to: &Path) -> Result {
    fs::copy(from, to).map(drop).map_err(|e| io(from, e))
}

fn write(path: &Path, data: &[u8]) -> Result {
    fs::write(path, data).map_err(|e| io(path, e))
}

fn io(path: &Path, e: std::io::Error) -> String {
    format!("{}：{e}", PathBuf::from(path).display())
}

/// 当前提交号的前 7 位；不在 git 仓库里时为 `None`。
fn commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !commit.is_empty()).then_some(commit)
}
