//! Linux（Wayland / niri）平台实现。层级：L2。

#![cfg(target_os = "linux")]

mod autostart;
mod capture;
mod clipboard;
mod display;
mod event_loop;
mod inject;
mod instance;
mod keymap;
mod overlay;
mod permission;
mod update;
mod wayland;

pub use autostart::LinuxAutostart;
pub use capture::LinuxCapture;
pub use clipboard::LinuxClipboard;
pub use display::LinuxDisplays;
pub use event_loop::LinuxEventLoop;
pub use inject::LinuxInject;
pub use instance::LinuxInstanceLock;
pub use keymap::{evdev_to_hid, hid_to_evdev, keymap_text, lock_keys_from_xkb, wayland_key_to_hid};
pub use overlay::LinuxOverlay;
pub use permission::LinuxPermissions;
pub use update::LinuxUpdater;
pub use wayland::shared_file;
