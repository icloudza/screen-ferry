//! 把本机光标藏起来：光标停在一个 1×1 的窗口上，这个窗口的光标是全透明的，于是看不见。
//! 本机用户一动鼠标，光标离开这一个像素就会重新出现，不需要额外处理。
//!
//! 藏着的时候每隔一会儿看一眼：光标还停在那里、却不在这个窗口上（别的置顶窗口后来盖了上来），
//! 就把窗口重新放到最上面、再把光标放回去，让它重新用透明的光标。
//!
//! 窗口要有消息循环才能设置光标形状，所以它跑在自己的线程上。藏起和收起都同步等它做完：
//! 调用方紧接着注入的移动必须排在后面，否则光标刚移到入口，又被挪回屏幕中央。

use std::cell::Cell;
use std::sync::mpsc;
use std::thread;

use ferry_platform::{Error, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::StationsAndDesktops::{GetThreadDesktop, HDESK, SetThreadDesktop};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateCursor, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, GetSystemMetrics, HWND_TOPMOST, KillTimer, MSG, PostMessageW, PostQuitMessage,
    RegisterClassW, SM_CXCURSOR, SM_CXSCREEN, SM_CYCURSOR, SM_CYSCREEN, SMTO_ABORTIFHUNG, SW_HIDE,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SendMessageTimeoutW, SetCursorPos, SetTimer, SetWindowPos,
    ShowWindow, TranslateMessage, WM_APP, WM_DESTROY, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WindowFromPoint,
};
use windows::core::w;

/// 把窗口放到屏幕中央、光标移上去。
const WM_HIDE_CURSOR: u32 = WM_APP + 10;
/// 收起窗口。光标已经被移走时它本来就可见。
const WM_SHOW_CURSOR: u32 = WM_APP + 11;
/// 销毁窗口，结束线程。
const WM_STOP: u32 = WM_APP + 12;
/// 等隐藏窗口处理完一条消息最多等这么久（毫秒）。正常情况下不到一毫秒。
const SEND_TIMEOUT_MS: u32 = 200;
/// 藏着的时候多久看一眼（毫秒）。
const CHECK_MS: u32 = 250;
const CHECK_TIMER: usize = 1;

thread_local! {
    /// 藏着的时候光标停在哪里；没藏时为 `None`。只在窗口所在的线程上读写。
    static PARKED: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
}

#[derive(Debug)]
pub(crate) struct CursorHider {
    /// 窗口句柄本身不是 Send，以整数保存；给窗口发消息可以在任何线程进行。
    hwnd: isize,
}

impl CursorHider {
    pub(crate) fn start() -> Result<Self> {
        // SAFETY: 借用创建方线程的桌面。创建方在 hider 存活期间保留自己的桌面绑定。
        // 新线程先显式绑定，不能依赖进程的 Default 桌面；安全输入工作线程可能在 Winlogon 上。
        let desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) }
            .map_err(|e| os("GetThreadDesktop", e))?
            .0 as isize;
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("ferry-cursor".into())
            .spawn(move || {
                // SAFETY: 本线程尚未创建窗口或钩子，借用创建方仍存活的桌面句柄。
                if let Err(e) = unsafe { SetThreadDesktop(HDESK(desktop as *mut _)) } {
                    let _ = tx.send(Err(os("SetThreadDesktop", e)));
                    return;
                }
                let hwnd = match create_window() {
                    Ok(hwnd) => hwnd,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                let _ = tx.send(Ok(hwnd.0 as isize));
                let mut msg = MSG::default();
                // SAFETY: 标准的消息循环，msg 指向有效的缓冲区。
                unsafe {
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            })
            .map_err(|e| Error::Os {
                call: "thread::spawn",
                detail: e.to_string(),
            })?;
        let hwnd = rx.recv().map_err(|_| Error::Os {
            call: "CreateWindowExW",
            detail: "隐藏光标的线程意外退出".into(),
        })??;
        Ok(Self { hwnd })
    }

    /// 藏起或收起，返回时已经做完。
    pub(crate) fn set_hidden(&self, hidden: bool) -> Result<()> {
        let msg = if hidden {
            WM_HIDE_CURSOR
        } else {
            WM_SHOW_CURSOR
        };
        // SAFETY: 窗口在本对象存在期间一直存在；不需要结果，结果指针为空。
        let done = unsafe {
            SendMessageTimeoutW(
                HWND(self.hwnd as *mut _),
                msg,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                SEND_TIMEOUT_MS,
                None,
            )
        };
        if done.0 == 0 {
            return Err(Error::Os {
                call: "SendMessageTimeoutW",
                detail: "隐藏光标的窗口没有及时响应".into(),
            });
        }
        Ok(())
    }

    fn post(&self, msg: u32) -> Result<()> {
        // SAFETY: 窗口在本对象存在期间一直存在。
        unsafe { PostMessageW(Some(HWND(self.hwnd as *mut _)), msg, WPARAM(0), LPARAM(0)) }
            .map_err(|e| os("PostMessageW", e))
    }
}

impl Drop for CursorHider {
    fn drop(&mut self) {
        let _ = self.post(WM_STOP);
    }
}

/// 在当前线程创建隐藏用的窗口：1×1、置顶、不在任务栏、不抢焦点，类光标是全透明的。
fn create_window() -> Result<HWND> {
    // SAFETY: 参数都是有效的常量、刚取得的句柄或在调用期间有效的缓冲区。
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| os("GetModuleHandleW", e))?;
        let (width, height) = (
            GetSystemMetrics(SM_CXCURSOR).max(1),
            GetSystemMetrics(SM_CYCURSOR).max(1),
        );
        // 单色位图每行按 16 位对齐。AND 全 1、XOR 全 0：每个像素都透明。
        let stride = (width as usize).div_ceil(16) * 2;
        let and_mask = vec![0xFF_u8; stride * height as usize];
        let xor_mask = vec![0_u8; stride * height as usize];
        let blank = CreateCursor(
            Some(instance.into()),
            0,
            0,
            width,
            height,
            and_mask.as_ptr().cast(),
            xor_mask.as_ptr().cast(),
        )
        .map_err(|e| os("CreateCursor", e))?;

        let class = w!("ScreenFerryCursorHider");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            hCursor: blank,
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|e| os("CreateWindowExW", e))
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: 都是对本线程自己的窗口和光标的操作。
    unsafe {
        match msg {
            WM_HIDE_CURSOR => {
                let (x, y) = (
                    GetSystemMetrics(SM_CXSCREEN) / 2,
                    GetSystemMetrics(SM_CYSCREEN) / 2,
                );
                park(hwnd, x, y);
                PARKED.with(|p| p.set(Some((x, y))));
                SetTimer(Some(hwnd), CHECK_TIMER, CHECK_MS, None);
                LRESULT(0)
            }
            WM_SHOW_CURSOR => {
                PARKED.with(|p| p.set(None));
                let _ = KillTimer(Some(hwnd), CHECK_TIMER);
                let _ = ShowWindow(hwnd, SW_HIDE);
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == CHECK_TIMER => {
                let Some((x, y)) = PARKED.with(Cell::get) else {
                    let _ = KillTimer(Some(hwnd), CHECK_TIMER);
                    return LRESULT(0);
                };
                let mut now = POINT::default();
                if GetCursorPos(&mut now).is_ok() && (now.x, now.y) != (x, y) {
                    // 光标被挪走了（多半是本机的人自己动了鼠标）：它本来就该出现，不再看着。
                    PARKED.with(|p| p.set(None));
                    let _ = KillTimer(Some(hwnd), CHECK_TIMER);
                } else if WindowFromPoint(POINT { x, y }) != hwnd {
                    // 别的置顶窗口盖了上来，光标用的是它的样子。
                    park(hwnd, x, y);
                }
                LRESULT(0)
            }
            WM_STOP => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// 把窗口放到最上面的 (x, y)，光标移上去。移动光标会让窗口收到 WM_SETCURSOR，换成透明的光标。
///
/// # Safety
///
/// `hwnd` 必须是本线程创建的隐藏窗口。
unsafe fn park(hwnd: HWND, x: i32, y: i32) {
    // SAFETY: 调用方保证窗口有效且属于本线程。
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            1,
            1,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = SetCursorPos(x, y);
    }
}

fn os(call: &'static str, e: windows::core::Error) -> Error {
    Error::Os {
        call,
        detail: e.message(),
    }
}
