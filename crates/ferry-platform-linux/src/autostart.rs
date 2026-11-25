//! 通过 freedesktop Desktop Entry 管理当前用户的开机自启。

use std::env;
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ferry_platform::{Autostart, Error, Result};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 当前用户的登录自启登记。
pub struct LinuxAutostart {
    command: Vec<String>,
    path: PathBuf,
}

impl LinuxAutostart {
    pub fn new(command: Vec<String>) -> Result<Self> {
        if command.first().is_none_or(|program| program.is_empty()) {
            return Err(Error::Failed("开机自启命令不能为空".into()));
        }
        if command.iter().any(|argument| {
            argument
                .chars()
                .any(|character| matches!(character, '\0' | '\n' | '\r'))
        }) {
            return Err(Error::Failed("开机自启命令包含非法控制字符".into()));
        }
        let config_home = match env::var_os("XDG_CONFIG_HOME") {
            Some(value) if !value.is_empty() && Path::new(&value).is_absolute() => {
                PathBuf::from(value)
            }
            _ => env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
                .ok_or_else(|| Error::Failed("无法确定用户配置目录".into()))?,
        };
        Ok(Self {
            command,
            path: config_home.join("autostart/screenferry.desktop"),
        })
    }
}

impl Autostart for LinuxAutostart {
    fn is_enabled(&self) -> Result<bool> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(io_error("read_to_string", error)),
        };
        let mut in_entry = false;
        let mut has_exec = false;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_entry {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (key.trim(), value.trim()) {
                ("Exec", _) => has_exec = true,
                ("Hidden", "true") => return Ok(false),
                ("X-GNOME-Autostart-enabled", "false") => return Ok(false),
                _ => {}
            }
        }
        Ok(has_exec)
    }

    fn set_enabled(&self, enabled: bool) -> Result<()> {
        if !enabled {
            return match fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(io_error("remove_file", error)),
            };
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| Error::Failed("开机自启路径没有父目录".into()))?;
        let mut builder = DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder
            .create(parent)
            .map_err(|error| io_error("create_dir_all", error))?;

        let exec = self
            .command
            .iter()
            .map(|argument| quote_exec_argument(argument))
            .collect::<Vec<_>>()
            .join(" ");
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=ScreenFerry\nExec={exec}\nTerminal=false\n"
        );
        write_atomic(&self.path, content.as_bytes())
    }
}

fn quote_exec_argument(argument: &str) -> String {
    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');
    for character in argument.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '$' => quoted.push_str("\\$"),
            '`' => quoted.push_str("\\`"),
            '%' => quoted.push_str("%%"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Failed("无效的自启路径".into()))?;
    let name = path
        .file_name()
        .ok_or_else(|| Error::Failed("无效的自启文件名".into()))?;
    for _ in 0..16 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error("open", error)),
        };
        let result = (|| {
            file.write_all(content)
                .map_err(|error| io_error("write", error))?;
            file.sync_all()
                .map_err(|error| io_error("sync_all", error))?;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
                .map_err(|error| io_error("set_permissions", error))?;
            fs::rename(&temporary, path).map_err(|error| io_error("rename", error))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        return result;
    }
    Err(Error::Failed("无法创建唯一的自启临时文件".into()))
}

fn io_error(call: &'static str, error: io::Error) -> Error {
    Error::Os {
        call,
        detail: error.to_string(),
    }
}
