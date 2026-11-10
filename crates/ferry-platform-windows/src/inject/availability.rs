//! 检查输入桌面和前台程序权限。只读检查，不切换桌面、不提升权限。

use ferry_platform::{Error, Result};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::StationsAndDesktops::{
    GetThreadDesktop, GetUserObjectInformationW, UOI_IO,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThreadId, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use windows::core::BOOL;

use crate::display::last_error;

pub(super) fn check() -> Result<()> {
    // SAFETY: 借用当前线程的桌面句柄；不关闭、不改变它。输出指向完整的 BOOL 缓冲区。
    let active = unsafe {
        let desktop =
            GetThreadDesktop(GetCurrentThreadId()).map_err(|_| last_error("GetThreadDesktop"))?;
        let mut active = BOOL(0);
        GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_IO,
            Some((&mut active as *mut BOOL).cast()),
            size_of::<BOOL>() as u32,
            None,
        )
        .map_err(|_| last_error("GetUserObjectInformationW"))?;
        active.as_bool()
    };
    if !active {
        return Err(Error::PermissionDenied(
            "Windows 正在锁屏或显示安全桌面（如 UAC 提示），请先在这台电脑上完成操作",
        ));
    }

    // SAFETY: 只读查询本进程令牌。伪句柄属于系统，不能 CloseHandle。
    if unsafe { elevated(GetCurrentProcess()) } != Some(false) {
        return Ok(());
    }
    // SAFETY: 只读查询前台窗口及其所属进程，不修改窗口状态。
    let blocked = unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            // 受保护进程可能不允许查询；不能据此断定它拒绝输入，交给 SendInput 的结果判断。
            return Ok(());
        };
        let elevated = elevated(process);
        let _ = CloseHandle(process);
        elevated == Some(true)
    };
    if blocked {
        return Err(Error::PermissionDenied(
            "前台程序以管理员身份运行，当前 ScreenFerry 无法控制它；请在这台电脑上切回普通窗口，或以管理员身份重新启动 ScreenFerry（仍不支持安全桌面）",
        ));
    }
    Ok(())
}

/// 本进程是不是以管理员身份运行。查不出来时当作不是。
pub(super) fn current_elevated() -> bool {
    // SAFETY: 只读查询本进程令牌。伪句柄属于系统，不能 CloseHandle。
    unsafe { elevated(GetCurrentProcess()) == Some(true) }
}

/// 查询进程是否已提升；不可查询时不猜测权限。每次打开的令牌都在返回前关闭。
unsafe fn elevated(process: HANDLE) -> Option<bool> {
    // SAFETY: process 是调用方提供的有效进程句柄；只申请查询权限。输出缓冲区大小与类型一致。
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut elevation = TOKEN_ELEVATION::default();
        let mut needed = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut needed,
        );
        let _ = CloseHandle(token);
        result.ok().map(|()| elevation.TokenIsElevated != 0)
    }
}
