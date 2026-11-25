//! Linux Wayland 会话、输入协议与 wl-clipboard 可用性检查。

use std::env;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use ferry_platform::{Error, Permissions, Result};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::wayland::{connect, error};

/// Wayland 平台的权限检查器。
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxPermissions;

impl LinuxPermissions {
    pub const fn new() -> Self {
        Self
    }
}

impl Permissions for LinuxPermissions {
    fn check_input(&self) -> Result<()> {
        if env::var_os("WAYLAND_DISPLAY").is_none_or(|value| value.is_empty()) {
            return Err(Error::Unsupported("当前会话不是 Wayland 桌面"));
        }
        let runtime = env::var_os("XDG_RUNTIME_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or(Error::PermissionDenied(
                "缺少 XDG_RUNTIME_DIR，无法连接 Wayland 桌面",
            ))?;
        if !runtime.is_dir() {
            return Err(Error::PermissionDenied("XDG_RUNTIME_DIR 不存在或不是目录"));
        }
        for command in ["wl-paste", "wl-copy"] {
            if !command_available(command) {
                return Err(Error::Unsupported(
                    "缺少 wl-paste 或 wl-copy，请安装 wl-clipboard",
                ));
            }
        }
        check_wayland_protocols()
    }
}

fn check_wayland_protocols() -> Result<()> {
    let connection = connect()?;
    let mut queue = connection.new_event_queue();
    let handle = queue.handle();
    let mut state = PermissionState::default();
    let _registry = connection.display().get_registry(&handle, ());
    queue
        .roundtrip(&mut state)
        .map_err(|e| error("roundtrip", e))?;
    const REQUIRED: &[&str] = &[
        "wl_compositor",
        "wl_shm",
        "wl_seat",
        "wl_output",
        "zwlr_layer_shell_v1",
        "zwp_pointer_constraints_v1",
        "zwp_relative_pointer_manager_v1",
        "zwlr_virtual_pointer_manager_v1",
        "zwp_virtual_keyboard_manager_v1",
    ];
    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|required| !state.interfaces.iter().any(|name| name == required))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Failed(format!(
            "当前 Wayland 合成器缺少 ScreenFerry 所需协议：{}",
            missing.join("、")
        )))
    }
}

#[derive(Default)]
struct PermissionState {
    interfaces: Vec<String>,
}

impl Dispatch<WlRegistry, ()> for PermissionState {
    fn event(
        state: &mut Self,
        _: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { interface, .. } = event {
            state.interfaces.push(interface);
        }
    }
}

fn command_available(command: &str) -> bool {
    env::split_paths(&env::var_os("PATH").unwrap_or_default()).any(|directory| {
        let Ok(path) = CString::new(directory.join(command).as_os_str().as_bytes()) else {
            return false;
        };
        // SAFETY: `path` 是有效的以 NUL 结尾的 C 字符串，目录文件描述符和标志均为常量。
        unsafe { libc::faccessat(libc::AT_FDCWD, path.as_ptr(), libc::X_OK, libc::AT_EACCESS) == 0 }
    })
}
