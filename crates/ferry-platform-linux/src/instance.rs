use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use ferry_platform::{Error, Result};

pub struct LinuxInstanceLock {
    _file: File,
}

impl LinuxInstanceLock {
    pub fn acquire(path: &Path) -> Result<Option<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| Error::Os {
                call: "open instance lock",
                detail: error.to_string(),
            })?;
        // SAFETY: file 是当前进程打开的有效文件描述符，flock 不修改内存。
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(Error::Os {
                call: "flock",
                detail: error.to_string(),
            });
        }
        Ok(Some(Self { _file: file }))
    }
}
