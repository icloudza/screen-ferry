//! Wayland 连接、自定义协议和事件分发。

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use wayland_client::Connection;

use ferry_platform::Error;

pub(crate) const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) mod virtual_keyboard {
    #[allow(clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::{wl_seat, wl_seat::WlSeat};
    use wayland_client::{Dispatch, QueueHandle};

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("src/protocols/virtual-keyboard-unstable-v1.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("src/protocols/virtual-keyboard-unstable-v1.xml");

    pub use self::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
    pub use self::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;

    pub fn create(
        manager: &ZwpVirtualKeyboardManagerV1,
        seat: &WlSeat,
        qh: &QueueHandle<impl Dispatch<ZwpVirtualKeyboardV1, ()> + 'static>,
    ) -> ZwpVirtualKeyboardV1 {
        manager.create_virtual_keyboard(seat, qh, ())
    }
}

pub(crate) mod virtual_pointer {
    #[allow(clippy::single_component_path_imports)]
    use wayland_client;
    use wayland_client::protocol::{wl_output, wl_seat, wl_seat::WlSeat};
    use wayland_client::{Dispatch, QueueHandle};

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("src/protocols/wlr-virtual-pointer-unstable-v1.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("src/protocols/wlr-virtual-pointer-unstable-v1.xml");

    pub use self::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;
    pub use self::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

    pub fn create(
        manager: &ZwlrVirtualPointerManagerV1,
        seat: &WlSeat,
        qh: &QueueHandle<impl Dispatch<ZwlrVirtualPointerV1, ()> + 'static>,
    ) -> ZwlrVirtualPointerV1 {
        manager.create_virtual_pointer(Some(seat), qh, ())
    }
}

pub(crate) fn error(call: &'static str, detail: impl std::fmt::Display) -> Error {
    Error::Os {
        call,
        detail: detail.to_string(),
    }
}

pub(crate) fn connect() -> Result<Connection, Error> {
    Connection::connect_to_env().map_err(|e| error("connect_to_env", e))
}

#[derive(Clone)]
pub(crate) struct Wake {
    read: std::sync::Arc<OwnedFd>,
    write: std::sync::Arc<OwnedFd>,
}

impl Wake {
    pub(crate) fn new() -> io::Result<Self> {
        let mut fds = [0; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            read: std::sync::Arc::new(unsafe { OwnedFd::from_raw_fd(fds[0]) }),
            write: std::sync::Arc::new(unsafe { OwnedFd::from_raw_fd(fds[1]) }),
        })
    }

    pub(crate) fn notify(&self) {
        let _ = unsafe { libc::write(self.write.as_raw_fd(), b"x".as_ptr().cast(), 1) };
    }

    pub(crate) fn clear(&self) {
        let mut byte = [0u8; 64];
        while unsafe { libc::read(self.read.as_raw_fd(), byte.as_mut_ptr().cast(), byte.len()) } > 0
        {
        }
    }
}

pub(crate) fn wait(conn: &Connection, wake: &Wake) -> io::Result<()> {
    conn.flush().map_err(|error| match error {
        wayland_client::backend::WaylandError::Io(error) => error,
        error => io::Error::other(error),
    })?;
    let Some(guard) = conn.prepare_read() else {
        return Ok(());
    };
    let mut fds = [
        libc::pollfd {
            fd: conn.backend().poll_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: wake.read.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    loop {
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, 100) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        break;
    }
    let connection_events = fds[0].revents;
    if connection_events & libc::POLLNVAL != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Wayland 连接文件描述符无效",
        ));
    }
    if connection_events & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
        guard.read().map_err(|error| match error {
            wayland_client::backend::WaylandError::Io(error) => error,
            error => io::Error::other(error),
        })?;
    } else if connection_events != 0 {
        return Err(io::Error::other("Wayland 连接异常"));
    }
    if fds[1].revents & libc::POLLIN != 0 {
        wake.clear();
    }
    Ok(())
}

pub(crate) fn timestamp() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u32
}

static NEXT_SHARED_FILE: AtomicU64 = AtomicU64::new(0);

/// 创建要交给合成器 mmap 的临时文件：wl_shm 缓冲区和 XKB keymap 都走这里。
///
/// 合成器会以 `PROT_READ|PROT_WRITE` + `MAP_SHARED` 映射 wl_shm 的 fd，keymap 也需要
/// `PROT_READ`；句柄必须以读写方式打开。只写句柄会让 mmap 以 EACCES 失败，合成器随即
/// 按协议错误（wl_shm 的 invalid_fd）断开整个连接。调用方在 flush 之后删除路径即可，
/// 合成器持有的是自己收到的 fd 副本。
pub fn shared_file(bytes: &[u8], suffix: &str) -> Result<(std::fs::File, PathBuf), Error> {
    use std::io::{Seek, SeekFrom, Write};
    let path = std::env::temp_dir().join(format!(
        "screenferry-{}-{}-{}.{}",
        std::process::id(),
        timestamp(),
        NEXT_SHARED_FILE.fetch_add(1, Ordering::Relaxed),
        suffix,
    ));
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| error("shared_file", e))?;
    file.write_all(bytes).map_err(|e| error("shared_file", e))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| error("shared_file", e))?;
    Ok((file, path))
}
