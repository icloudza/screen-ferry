//! 输入注入：`InputInject` 的 Windows 实现，基于 `SendInput`。
//!
//! 绝对移动使用整个虚拟桌面的归一化坐标（0–65535），换算时落在目标像素的中心，避免差一像素。
//! 键盘用扫描码注入，不受键盘布局和输入法影响。
//! 光标在对方屏幕上时，本机光标停到一个看不见的位置（见 [`hider`]）。

mod availability;
pub(crate) mod hider;
mod service;

pub use service::{
    WinInject, authorize as authorize_input_peer, install as install_input_service,
    prepare as prepare_input_service, remove as remove_input_service, revoke as revoke_input_peer,
    run as run_input_service, run_agent as run_input_agent, stop as stop_input_service,
};

use ferry_platform::{Error, InputInject, Result};
use ferry_types::{InputEvent, KeyAction, KeyCode, LockKeys, MouseButton, Point};
use windows::Win32::Foundation::{ERROR_SUCCESS, GetLastError, SetLastError};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSE_EVENT_FLAGS,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, MOUSEINPUT, SendInput, VIRTUAL_KEY, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    XBUTTON1, XBUTTON2,
};

use crate::display::{ensure_dpi_aware, last_error};
use crate::keymap::{WinKey, win_key};
use hider::CursorHider;

/// 本机键鼠注入。
#[derive(Debug)]
struct LocalInject {
    /// 第一次需要隐藏光标时才创建。
    hider: Option<CursorHider>,
}

impl LocalInject {
    pub fn new() -> Self {
        ensure_dpi_aware();
        Self { hider: None }
    }
}

impl Default for LocalInject {
    fn default() -> Self {
        Self::new()
    }
}

impl InputInject for LocalInject {
    fn check_available(&mut self) -> Result<()> {
        availability::check()
    }

    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        match *event {
            InputEvent::PointerMotion { position } => {
                let (x, y) = normalize(position);
                send(mouse(
                    x,
                    y,
                    0,
                    MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                ))
            }
            InputEvent::PointerDelta { dx, dy } => send(mouse(dx, dy, 0, MOUSEEVENTF_MOVE)),
            InputEvent::Button {
                button, pressed, ..
            } => {
                let (flags, data) = button_flags(button, pressed)
                    .ok_or(Error::Unsupported("5 号以上的鼠标按键"))?;
                send(mouse(0, 0, data, flags))
            }
            InputEvent::Wheel { dx, dy, .. } => {
                // mouseData 是有符号数按位存放。
                if dy != 0 {
                    send(mouse(0, 0, dy as u32, MOUSEEVENTF_WHEEL))?;
                }
                if dx != 0 {
                    send(mouse(0, 0, dx as u32, MOUSEEVENTF_HWHEEL))?;
                }
                Ok(())
            }
            InputEvent::Key { code, action } => {
                let key = win_key(code).ok_or(Error::Unsupported("没有对应 Windows 按键的键"))?;
                send(keyboard(key, action == KeyAction::Up))
            }
        }
    }

    fn sync_locks(&mut self, wanted: LockKeys, mask: LockKeys) -> Result<()> {
        let locks = [
            (LockKeys::CAPS, VK_CAPITAL, KeyCode::CAPS_LOCK),
            (LockKeys::NUM, VK_NUMLOCK, KeyCode::NUM_LOCK),
            (LockKeys::SCROLL, VK_SCROLL, KeyCode::SCROLL_LOCK),
        ];
        for (lock, vk, code) in locks {
            if !mask.contains(lock) {
                continue;
            }
            // SAFETY: 只读查询键盘状态；最低位表示锁定键是否打开。
            let on = unsafe { GetKeyState(i32::from(vk.0)) } & 1 != 0;
            if on != wanted.contains(lock) {
                let key = win_key(code).ok_or(Error::Unsupported("锁定键"))?;
                send(keyboard(key, false))?;
                send(keyboard(key, true))?;
            }
        }
        Ok(())
    }

    fn set_cursor_hidden(&mut self, hidden: bool) -> Result<()> {
        let hider = match &self.hider {
            Some(hider) => hider,
            // 从没藏过，也就不需要再显示。
            None if !hidden => return Ok(()),
            None => self.hider.insert(CursorHider::start()?),
        };
        hider.set_hidden(hidden)
    }
}

fn keyboard(key: WinKey, up: bool) -> INPUT {
    let (vk, scan, mut flags) = match key {
        WinKey::Scan {
            code,
            extended: false,
        } => (VIRTUAL_KEY(0), code, KEYEVENTF_SCANCODE),
        WinKey::Scan {
            code,
            extended: true,
        } => (
            VIRTUAL_KEY(0),
            code,
            KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY,
        ),
        WinKey::Virtual(vk) => (VIRTUAL_KEY(vk), 0, KEYBD_EVENT_FLAGS(0)),
        WinKey::Media { vk, scan } => (VIRTUAL_KEY(vk), scan, KEYEVENTF_EXTENDEDKEY),
    };
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn mouse(dx: i32, dy: i32, data: u32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(input: INPUT) -> Result<()> {
    // SAFETY: 传入一个完整初始化的 INPUT，大小参数与之一致。
    // SendInput 被 UIPI 拦截时可能不设置错误代码，先清空，避免报出上一次调用遗留的错误。
    let sent = unsafe {
        SetLastError(ERROR_SUCCESS);
        SendInput(&[input], size_of::<INPUT>() as i32)
    };
    if sent == 1 {
        Ok(())
    } else {
        // SAFETY: 紧接 SendInput 读取本线程的错误，不做其他系统调用。
        if unsafe { GetLastError() } == ERROR_SUCCESS {
            Err(Error::Os {
                call: "SendInput",
                detail: "Windows 未接受输入，也未提供错误代码；可能受前台程序权限或输入限制影响"
                    .into(),
            })
        } else {
            Err(last_error("SendInput"))
        }
    }
}

fn button_flags(button: MouseButton, pressed: bool) -> Option<(MOUSE_EVENT_FLAGS, u32)> {
    let pick = |down, up| if pressed { down } else { up };
    Some(match button.number() {
        1 => (pick(MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP), 0),
        2 => (pick(MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP), 0),
        3 => (pick(MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP), 0),
        4 => (
            pick(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP),
            u32::from(XBUTTON1),
        ),
        5 => (
            pick(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP),
            u32::from(XBUTTON2),
        ),
        _ => return None,
    })
}

/// 虚拟桌面坐标 → `SendInput` 的归一化坐标。
fn normalize(p: Point) -> (i32, i32) {
    // SAFETY: 只读查询系统度量。
    let (left, top, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    (to_unit(p.x, left, width), to_unit(p.y, top, height))
}

/// 把 `[start, start + extent)` 中的像素映射到 0–65535，取像素中心。
fn to_unit(v: i32, start: i32, extent: i32) -> i32 {
    if extent <= 0 {
        return 0;
    }
    let offset = (i64::from(v) - i64::from(start)).clamp(0, i64::from(extent) - 1);
    ((2 * offset + 1) * 65_536 / (2 * i64::from(extent))) as i32
}
