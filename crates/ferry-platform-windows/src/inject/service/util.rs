//! Win32 句柄和错误的窄封装。

use ferry_platform::{Error, Result};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::System::Threading::WaitForSingleObject;

#[derive(Debug)]
pub(super) struct Handle(pub HANDLE);

// SAFETY: 独占拥有可跨线程使用的内核句柄；借用不跨线程，析构只关闭一次。
unsafe impl Send for Handle {}

impl Handle {
    pub fn new(handle: HANDLE) -> Result<Self> {
        if handle.is_invalid() {
            Err(crate::display::last_error("创建句柄"))
        } else {
            Ok(Self(handle))
        }
    }

    pub fn signalled(&self) -> bool {
        // SAFETY: 当前对象拥有有效的可等待句柄，零超时不阻塞。
        unsafe { WaitForSingleObject(self.0, 0) != WAIT_TIMEOUT }
    }

    pub fn wait(&self, ms: u32) -> bool {
        // SAFETY: 同上；仅用于事件和进程句柄。
        unsafe { WaitForSingleObject(self.0, ms) == WAIT_OBJECT_0 }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: 句柄唯一归本对象所有，不是伪句柄。
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub(super) fn os(call: &'static str, error: windows::core::Error) -> Error {
    Error::Os {
        call,
        detail: error.message(),
    }
}

pub(super) fn failure(message: &str) -> Error {
    Error::Failed(message.to_owned())
}

pub(super) fn ticks() -> u64 {
    // SAFETY: 无参数的只读单调时钟查询。
    unsafe { GetTickCount64() }
}
