//! 发布（ADR 0014）：
//!
//! - `release <版本>`：改版本号（Cargo.toml、app/pubspec.yaml、Cargo.lock），整理 CHANGELOG，
//!   检查 `docs/releases/<版本>.md` 里的更新说明，然后提交并打 tag。还没发过的当前版本也可以直接发（第一版）。
//!   版本号的规矩：只有修复升最后一位；有新功能、改进，或者改了两台之间的通信方式，升中间一位。推送由人来做，推上 tag 后
//!   CI（`.github/workflows/release.yml`）构建两个系统的安装包，存成 GitHub 上的草稿。
//! - `manifest`：CI 用。按目录里的更新包写 `update.json`：版本、发布和过期时间、各平台的地址、
//!   大小和 SHA-256、握手格式标记（固定为 1）、更新说明。
//! - `sign <文件>`：CI 用。用发布私钥给文件做 minisign 签名，写出 `<文件>.minisig`，再用程序内置的
//!   公钥校验一遍，确认签名的密钥是程序认的。私钥和口令来自环境变量
//!   `SCREENFERRY_UPDATE_KEY`（私钥文件的内容）、`SCREENFERRY_UPDATE_KEY_PASSWORD`。
//! - `update-keygen`：生成两把发布密钥（日常用、备用），私钥加口令后写到仓库以外的目录，
//!   两把公钥写进 `crates/ferry-update/src/keys.rs`。只需运行一次。

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ferry_update::{Asset, Manifest, Notes, RELEASE_KEYS, Release, SCHEMA};
use ring::digest::{Context, SHA256};

type Result<T = ()> = std::result::Result<T, String>;

/// 清单多久过期：过期后程序不再说“已是最新”。发布间隔超过它时要重新签一份清单。
const MANIFEST_LIFETIME: i64 = 180 * 86_400;

/// 各平台更新包的文件名结尾，对应清单里的平台名。
const PLATFORMS: [(&str, &str); 2] = [
    ("macos-universal", "-macos-universal.zip"),
    ("windows-x86_64", "-windows-x64-setup.exe"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask 在仓库根目录下")
        .to_path_buf()
}

// ── release ───────────────────────────────────

pub(crate) fn release(args: &[String]) -> Result {
    let [version] = args else {
        return Err("用法：cargo xtask release <版本>，例如 0.2.0".into());
    };
    let parsed =
        ferry_update::Version::parse(version).map_err(|e| format!("版本号 {version}：{e}"))?;
    let root = root();
    let tag = format!("v{version}");
    let tagged = Command::new("git")
        .args(["tag", "--list", &tag])
        .current_dir(&root)
        .output()
        .map_err(|e| format!("无法运行 git：{e}"))?;
    if !String::from_utf8_lossy(&tagged.stdout).trim().is_empty() {
        return Err(format!("{tag} 已经发过了"));
    }

    let notes_path = root.join(format!("docs/releases/{version}.md"));
    if !notes_path.exists() {
        if let Some(dir) = notes_path.parent() {
            fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
        }
        fs::write(&notes_path, notes_template(version)).map_err(|e| io(&notes_path, e))?;
        return Err(format!(
            "先在 {} 里写好中英文的更新说明，再运行一次。\n\
             每条一行、以“- ”开头，放在“### 新增 / 改进 / 修复”（英文 New / Improved / Fixed）下面，\n\
             没有内容的分类整个删掉",
            notes_path.display()
        ));
    }
    let notes = read_notes(&notes_path)?;
    if notes.zh.is_empty() || notes.en.is_empty() {
        return Err(format!(
            "{} 里的中文或英文更新说明是空的",
            notes_path.display()
        ));
    }

    // 版本号：比现在的新就改；和现在的一样（还没发过的第一版）就不动。
    let cargo_toml = root.join("Cargo.toml");
    let text = fs::read_to_string(&cargo_toml).map_err(|e| io(&cargo_toml, e))?;
    let current = workspace_version(&text)?;
    let current_parsed = ferry_update::Version::parse(&current).map_err(|e| e.to_string())?;
    let mut files = vec![
        "CHANGELOG.md".to_owned(),
        format!("docs/releases/{version}.md"),
    ];
    let mut build = None;
    if parsed < current_parsed {
        return Err(format!("{version} 比现在的 {current} 还旧"));
    }
    if parsed > current_parsed {
        let text = text.replacen(
            &format!("version = \"{current}\""),
            &format!("version = \"{version}\""),
            1,
        );
        fs::write(&cargo_toml, text).map_err(|e| io(&cargo_toml, e))?;

        let pubspec = root.join("app/pubspec.yaml");
        let text = fs::read_to_string(&pubspec).map_err(|e| io(&pubspec, e))?;
        let text: String = text
            .lines()
            .map(|line| match line.strip_prefix("version: ") {
                Some(old) => {
                    let next = old
                        .split('+')
                        .nth(1)
                        .and_then(|b| b.trim().parse::<u32>().ok())
                        .map_or(1, |b| b + 1);
                    build = Some(next);
                    format!("version: {version}+{next}")
                }
                None => line.to_owned(),
            })
            .map(|line| line + "\n")
            .collect();
        fs::write(&pubspec, text).map_err(|e| io(&pubspec, e))?;
        run(&root, "cargo", &["update", "--workspace", "--offline"])?;
        files.extend(["Cargo.toml", "Cargo.lock", "app/pubspec.yaml"].map(str::to_owned));
    }

    let changelog = root.join("CHANGELOG.md");
    let text = fs::read_to_string(&changelog).map_err(|e| io(&changelog, e))?;
    if !text.contains("## [Unreleased]") {
        return Err("CHANGELOG.md 里没有 ## [Unreleased]".into());
    }
    let text = text.replacen(
        "## [Unreleased]",
        &format!("## [Unreleased]\n\n## [{version}] - {}", today()),
        1,
    );
    fs::write(&changelog, text).map_err(|e| io(&changelog, e))?;

    let mut add = vec!["add"];
    add.extend(files.iter().map(String::as_str));
    run(&root, "git", &add)?;
    run(
        &root,
        "git",
        &["commit", "-m", &format!("release: {version}")],
    )?;
    run(
        &root,
        "git",
        &["tag", "-a", &tag, "-m", &format!("ScreenFerry {version}")],
    )?;
    match build {
        Some(build) => println!("\n✓ 已提交并打好 tag {tag}（设置界面的构建号 {build}）"),
        None => println!("\n✓ 已提交并打好 tag {tag}（版本号本来就是 {version}，没有改）"),
    }
    println!("  推送后 CI 会构建安装包、存成 GitHub 上的草稿：");
    println!("    git push && git push origin {tag}");
    Ok(())
}

/// 更新说明的分类：中文、英文的小标题一一对应，按这个顺序写。
const CATEGORIES: [(&str, &str); 3] = [("新增", "New"), ("改进", "Improved"), ("修复", "Fixed")];

fn notes_template(version: &str) -> String {
    let section = |english: bool| -> String {
        CATEGORIES
            .iter()
            .map(|(zh, en)| format!("### {}\n\n- \n\n", if english { en } else { zh }))
            .collect()
    };
    format!(
        "# ScreenFerry {version}\n\n## 简体中文\n\n{}## English\n\n{}",
        section(false),
        section(true)
    )
    .trim_end()
    .to_owned()
        + "\n"
}

/// `docs/releases/<版本>.md`：`## 简体中文`、`## English` 两节，每节里是 `### 分类` 和它下面以“- ”开头的条目。
/// 写进清单时保留这个结构（`### 分类` 一行，后面每条一行“- 条目”），程序按它分类显示。
fn read_notes(path: &Path) -> Result<Notes> {
    let text = fs::read_to_string(path).map_err(|e| io(path, e))?;
    let mut notes = Notes::default();
    let mut section: Option<bool> = None;
    let mut category: Option<&str> = None;
    let mut pending: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(title) = line.strip_prefix("## ") {
            section = match title.trim() {
                "简体中文" => Some(false),
                "English" => Some(true),
                _ => None,
            };
            category = None;
            continue;
        }
        let Some(english) = section else { continue };
        if let Some(title) = line.strip_prefix("### ") {
            let title = title.trim();
            let known = CATEGORIES
                .iter()
                .find(|(zh, en)| title == if english { *en } else { *zh });
            let Some((zh, en)) = known else {
                let allowed: Vec<&str> = CATEGORIES
                    .iter()
                    .map(|(zh, en)| if english { *en } else { *zh })
                    .collect();
                return Err(format!(
                    "{}：不认识的分类“{title}”，只能是 {}",
                    path.display(),
                    allowed.join("、")
                ));
            };
            category = Some(if english { en } else { zh });
            pending = Some(format!("### {}", category.unwrap_or_default()));
            continue;
        }
        let Some(item) = line
            .strip_prefix("- ")
            .map(str::trim)
            .filter(|i| !i.is_empty())
        else {
            continue;
        };
        if category.is_none() {
            return Err(format!(
                "{}：“{item}”要放在某个分类（### …）下面",
                path.display()
            ));
        }
        let target = if english {
            &mut notes.en
        } else {
            &mut notes.zh
        };
        // 分类的小标题等到它下面有条目时才写，空分类不出现。
        for line in pending.take().into_iter().chain([format!("- {item}")]) {
            if !target.is_empty() {
                target.push('\n');
            }
            target.push_str(&line);
        }
    }
    Ok(notes)
}

// ── manifest ──────────────────────────────────

/// `manifest --version 0.2.0 --dir dist --base-url https://…/releases/download/v0.2.0 --out dist/update.json`
pub(crate) fn manifest(args: &[String]) -> Result {
    let opt = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
            .ok_or_else(|| format!("缺少 {name}"))
    };
    let version = opt("--version")?;
    let dir = PathBuf::from(opt("--dir")?);
    let base = opt("--base-url")?.trim_end_matches('/').to_owned();
    let out = PathBuf::from(opt("--out")?);
    let root = root();
    let notes = read_notes(&root.join(format!("docs/releases/{version}.md")))?;

    let mut assets = std::collections::BTreeMap::new();
    for entry in fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
        let path = entry.map_err(|e| io(&dir, e))?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some((platform, _)) = PLATFORMS.iter().find(|(_, suffix)| name.ends_with(suffix))
        else {
            continue;
        };
        if !name.contains(&format!("-{version}-")) {
            return Err(format!("{name} 的版本号不是 {version}"));
        }
        let (size, sha256) = hash(&path)?;
        println!("  {platform}: {name}（{size} 字节）");
        assets.insert(
            (*platform).to_owned(),
            Asset {
                url: format!("{base}/{name}"),
                size,
                sha256,
            },
        );
    }
    if assets.is_empty() {
        return Err(format!("{} 里没有更新包", dir.display()));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let manifest = Manifest {
        schema: SCHEMA,
        version,
        published: Manifest::format_time(now),
        expires: Manifest::format_time(now + MANIFEST_LIFETIME),
        protocol: ferry_proto::WIRE_FORMAT,
        notes,
        assets,
    };
    Release::from_manifest(manifest.clone()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(&out, json + "\n").map_err(|e| io(&out, e))?;
    println!("✓ {}", out.display());
    Ok(())
}

fn hash(path: &Path) -> Result<(u64, String)> {
    let data = fs::read(path).map_err(|e| io(path, e))?;
    let mut ctx = Context::new(&SHA256);
    ctx.update(&data);
    let hex = ctx
        .finish()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((data.len() as u64, hex))
}

// ── sign ──────────────────────────────────────

pub(crate) fn sign(args: &[String]) -> Result {
    let [file] = args else {
        return Err("用法：cargo xtask sign <文件>".into());
    };
    let key = std::env::var("SCREENFERRY_UPDATE_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .ok_or("没有发布私钥：环境变量 SCREENFERRY_UPDATE_KEY")?;
    let password = std::env::var("SCREENFERRY_UPDATE_KEY_PASSWORD").ok();
    let secret = minisign::SecretKeyBox::from_string(&key)
        .and_then(|b| b.into_secret_key(password))
        .map_err(|e| format!("读不了发布私钥（口令对吗？）：{e}"))?;
    let path = PathBuf::from(file);
    let data = fs::read(&path).map_err(|e| io(&path, e))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let signature = minisign::sign(None, &secret, Cursor::new(&data), Some(name), None)
        .map_err(|e| format!("签名失败：{e}"))?
        .into_string();
    let out = PathBuf::from(format!("{file}.minisig"));
    fs::write(&out, &signature).map_err(|e| io(&out, e))?;

    // 用程序内置的公钥校验一遍：签名的密钥不是程序认的，发出去也没人能更新。
    Release::verify(&data, &signature, RELEASE_KEYS).map_err(|e| {
        format!("签好了，但程序内置的公钥校验不过（{e}）：发布私钥和 keys.rs 里的公钥对不上")
    })?;
    println!("✓ {}", out.display());
    Ok(())
}

// ── update-keygen ─────────────────────────────

pub(crate) fn keygen(args: &[String]) -> Result {
    let out = match args {
        [] => home()?.join("ScreenFerry-release-keys"),
        [flag, dir] if flag == "--out" => PathBuf::from(dir),
        _ => return Err("用法：cargo xtask update-keygen [--out <目录>]".into()),
    };
    let root = root();
    if out.starts_with(&root) {
        return Err("私钥不能放在仓库里，换一个目录".into());
    }
    if !RELEASE_KEYS.is_empty() {
        return Err("keys.rs 里已经有发布公钥了。轮换密钥见 ADR 0014，不要直接覆盖".into());
    }
    fs::create_dir_all(&out).map_err(|e| io(&out, e))?;
    let mut publics = Vec::new();
    for (name, label) in [("release", "日常发布用"), ("backup", "备用，私钥离线保存")]
    {
        let key_path = out.join(format!("screenferry-{name}.key"));
        if key_path.exists() {
            return Err(format!("{} 已经存在，不覆盖", key_path.display()));
        }
        println!("\n为“{label}”的私钥设一个口令（输入时不显示）：");
        let pair = minisign::KeyPair::generate_encrypted_keypair(None)
            .map_err(|e| format!("生成密钥失败：{e}"))?;
        let secret = pair
            .sk
            .to_box(Some(&format!("ScreenFerry {label}")))
            .map_err(|e| e.to_string())?
            .into_string();
        write_private(&key_path, &secret)?;
        let public = pair.pk.to_base64();
        let pub_path = out.join(format!("screenferry-{name}.pub"));
        fs::write(&pub_path, format!("{public}\n")).map_err(|e| io(&pub_path, e))?;
        publics.push((public, label));
    }

    let keys = root.join("crates/ferry-update/src/keys.rs");
    let text = fs::read_to_string(&keys).map_err(|e| io(&keys, e))?;
    let (start, end) = ("// update-keygen：开始", "// update-keygen：结束");
    let (Some(a), Some(b)) = (text.find(start), text.find(end)) else {
        return Err("keys.rs 里找不到 update-keygen 的标记".into());
    };
    let lines: String = publics
        .iter()
        .map(|(key, label)| format!("    // {label}，生成于 {}\n    \"{key}\",\n", today()))
        .collect();
    let text = format!("{}{start}\n{lines}    {}", &text[..a], &text[b..]);
    fs::write(&keys, text).map_err(|e| io(&keys, e))?;

    println!(
        "\n✓ 私钥在 {}，两把公钥已写进 {}",
        out.display(),
        keys.display()
    );
    println!("  接下来：");
    println!("  1. 在 GitHub 仓库的 Settings → Secrets and variables → Actions 里新建：");
    println!("       SCREENFERRY_UPDATE_KEY           screenferry-release.key 的全部内容");
    println!("       SCREENFERRY_UPDATE_KEY_PASSWORD  它的口令");
    println!("  2. 把 screenferry-backup.key 和它的口令存进密码管理器，然后从这台电脑上删掉。");
    println!("  3. 提交 keys.rs 的改动。");
    Ok(())
}

/// 写私钥文件：只有自己能读。
fn write_private(path: &Path, text: &str) -> Result {
    fs::write(path, text).map_err(|e| io(path, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| io(path, e))?;
    }
    Ok(())
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "找不到用户目录".into())
}

// ── 小工具 ────────────────────────────────────

fn workspace_version(text: &str) -> Result<String> {
    let manifest: toml::Table = text.parse().map_err(|e| format!("Cargo.toml：{e}"))?;
    manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "Cargo.toml 中没有 [workspace.package] version".into())
}

/// 今天的日期（UTC），例如 `2026-11-02`。
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn run(dir: &Path, program: &str, args: &[&str]) -> Result {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("无法运行 {program}：{e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} 失败", args.join(" ")))
    }
}

fn io(path: &Path, e: std::io::Error) -> String {
    format!("{}：{e}", path.display())
}
