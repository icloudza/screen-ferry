//! 输入注入：`InputInject` 的 macOS 实现，基于 CGEventPost。
//!
//! 事件从 HID 层发出，系统和各个程序都把它当成真实键鼠：
//! - 移动时按着鼠标键就发“拖动”而不是“移动”，并带上位移，否则拖不动窗口；
//! - 鼠标键带上连击次数，双击、三击才认得出来；
//! - 按键带上此刻按着的修饰键，修饰键自己以“修饰键变化”发出，和真实键盘一样；
//! - 音量、播放键以系统事件发出，普通按键事件调不了音量；
//! - Caps Lock 发按键改不了大小写状态，直接改系统的锁定状态（见 [`caps_lock`]）。
//!
//! 程序发出事件后，系统默认在 0.25 秒内压住本机真实的键鼠。被控制时本机的人也可能在用
//! 触控板，所以把这段压制关掉。
//!
//! 光标在对方屏幕上时隐藏本机光标（见 [`cursor`]）。

mod caps_lock;
pub(crate) mod cursor;

use std::time::{Duration, Instant};

use ferry_platform::{Error, InputInject, Result};
use ferry_types::{InputEvent, KeyAction, KeyCode, LockKeys, MouseButton, Point, WHEEL_NOTCH};
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_core_foundation::{CFRetained, CGPoint};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFilterMask, CGEventFlags, CGEventSource, CGEventSourceStateID,
    CGEventSuppressionState, CGEventTapLocation, CGEventType, CGMouseButton, CGScrollEventUnit,
};

use crate::capture::live_cursor;
use crate::keymap::{mac_keycode, modifier_flag, nx_key};
use caps_lock::CapsLock;
use cursor::CursorHider;

/// 系统事件中“辅助控制键”（媒体键）的子类型 `NX_SUBTYPE_AUX_CONTROL_BUTTONS`。
const AUX_CONTROL_BUTTONS: i16 = 8;
/// 两次点击离得不超过这么多个点，才算连击。
const CLICK_SLOP: f64 = 4.0;

/// 本机键鼠注入。
pub struct MacInject {
    source: Source,
    /// 最近一次注入的光标位置。点击和滚动在这里发生。
    at: CGPoint,
    /// 按着的鼠标键，第 n 位对应协议中的 n 号键。
    buttons: u32,
    /// 按着的修饰键（macOS 虚拟键码）。
    modifiers: Vec<u16>,
    /// 上一次按下鼠标键，用来数连击。
    last_click: Option<Click>,
    /// 滚轮换算成行之后不足一行的余数，累积到下一次。
    wheel_carry: (i32, i32),
    /// 第一次需要时才打开。
    caps_lock: Option<CapsLock>,
    cursor: CursorHider,
}

/// 可以随注入对象一起交给别的线程的事件源。
struct Source(CFRetained<CGEventSource>);

// SAFETY: CGEventSource 是 CoreFoundation 对象，引用计数可以跨线程；
// 它只在持有 `MacInject` 的那个线程上使用，不会同时被两个线程访问。
unsafe impl Send for Source {}

#[derive(Clone, Copy)]
struct Click {
    button: u8,
    count: i64,
    at: CGPoint,
    time: Instant,
}

impl MacInject {
    pub fn new() -> Result<Self> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok_or(Error::Os {
            call: "CGEventSourceCreate",
            detail: "返回空".into(),
        })?;
        CGEventSource::set_local_events_suppression_interval(Some(&source), 0.0);
        CGEventSource::set_local_events_filter_during_suppression_state(
            Some(&source),
            CGEventFilterMask::all(),
            CGEventSuppressionState::EventSuppressionStateSuppressionInterval,
        );
        let at = live_cursor().map_or(CGPoint::new(0.0, 0.0), |p| {
            CGPoint::new(f64::from(p.x), f64::from(p.y))
        });
        Ok(Self {
            source: Source(source),
            at,
            buttons: 0,
            modifiers: Vec::new(),
            last_click: None,
            wheel_carry: (0, 0),
            caps_lock: None,
            cursor: CursorHider::new(),
        })
    }

    fn post(&self, event: &CGEvent) {
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event));
    }

    fn mouse_event(&self, ty: CGEventType, button: CGMouseButton) -> Result<CFRetained<CGEvent>> {
        CGEvent::new_mouse_event(Some(&self.source.0), ty, self.at, button).ok_or(Error::Os {
            call: "CGEventCreateMouseEvent",
            detail: "返回空".into(),
        })
    }

    fn move_to(&mut self, to: CGPoint) -> Result<()> {
        let (dx, dy) = (to.x - self.at.x, to.y - self.at.y);
        self.at = to;
        // 按着鼠标键时发“拖动”：按着左键优先，其次右键，再次其他键里编号最小的那个。
        let (ty, button) = match self.held_button() {
            Some(1) => (CGEventType::LeftMouseDragged, CGMouseButton::Left),
            Some(2) => (CGEventType::RightMouseDragged, CGMouseButton::Right),
            Some(n) => (
                CGEventType::OtherMouseDragged,
                CGMouseButton(u32::from(n) - 1),
            ),
            None => (CGEventType::MouseMoved, CGMouseButton::Left),
        };
        let event = self.mouse_event(ty, button)?;
        let set = |field, v: f64| CGEvent::set_integer_value_field(Some(&event), field, v as i64);
        set(CGEventField::MouseEventDeltaX, dx.round());
        set(CGEventField::MouseEventDeltaY, dy.round());
        self.post(&event);
        Ok(())
    }

    fn held_button(&self) -> Option<u8> {
        if self.buttons & (1 << 1) != 0 {
            Some(1)
        } else if self.buttons & (1 << 2) != 0 {
            Some(2)
        } else {
            (3..32).find(|n| self.buttons & (1 << n) != 0)
        }
    }

    fn button(&mut self, button: MouseButton, pressed: bool) -> Result<()> {
        let n = button.number();
        let (down, up, which) = match n {
            1 => (
                CGEventType::LeftMouseDown,
                CGEventType::LeftMouseUp,
                CGMouseButton::Left,
            ),
            2 => (
                CGEventType::RightMouseDown,
                CGEventType::RightMouseUp,
                CGMouseButton::Right,
            ),
            3..32 => (
                CGEventType::OtherMouseDown,
                CGEventType::OtherMouseUp,
                CGMouseButton(u32::from(n) - 1),
            ),
            _ => return Err(Error::Unsupported("32 号以上的鼠标按键")),
        };
        let count = if pressed {
            self.buttons |= 1 << n;
            self.count_click(n)
        } else {
            self.buttons &= !(1 << n);
            self.last_click.map_or(1, |c| c.count)
        };
        let event = self.mouse_event(if pressed { down } else { up }, which)?;
        CGEvent::set_integer_value_field(Some(&event), CGEventField::MouseEventClickState, count);
        self.post(&event);
        Ok(())
    }

    /// 这次按下是第几连击：同一个键、在系统的双击间隔内、几乎同一个位置。
    fn count_click(&mut self, button: u8) -> i64 {
        let now = Instant::now();
        let interval = Duration::from_secs_f64(NSEvent::doubleClickInterval().max(0.0));
        let count = match self.last_click {
            Some(last)
                if last.button == button
                    && now.duration_since(last.time) <= interval
                    && (last.at.x - self.at.x).abs() <= CLICK_SLOP
                    && (last.at.y - self.at.y).abs() <= CLICK_SLOP =>
            {
                last.count + 1
            }
            _ => 1,
        };
        self.last_click = Some(Click {
            button,
            count,
            at: self.at,
            time: now,
        });
        count
    }

    fn wheel(&mut self, dx: i32, dy: i32) -> Result<()> {
        // 整行交给只认整数的程序，余数留到下一次；带小数的那一份给按精确值滚动的程序。
        let whole = |carry: &mut i32, delta: i32| {
            *carry += delta;
            let lines = *carry / WHEEL_NOTCH;
            *carry -= lines * WHEEL_NOTCH;
            lines
        };
        let (mut carry_x, mut carry_y) = self.wheel_carry;
        let lines_y = whole(&mut carry_y, dy);
        let lines_x = whole(&mut carry_x, dx);
        self.wheel_carry = (carry_x, carry_y);

        // 协议里 dy 为正向上、dx 为正向右；macOS 第一轴为正向上，第二轴为正向左。
        let event = CGEvent::new_scroll_wheel_event2(
            Some(&self.source.0),
            CGScrollEventUnit::Line,
            2,
            lines_y,
            -lines_x,
            0,
        )
        .ok_or(Error::Os {
            call: "CGEventCreateScrollWheelEvent2",
            detail: "返回空".into(),
        })?;
        let notch = f64::from(WHEEL_NOTCH);
        let set = |field, v| CGEvent::set_double_value_field(Some(&event), field, v);
        set(
            CGEventField::ScrollWheelEventFixedPtDeltaAxis1,
            f64::from(dy) / notch,
        );
        set(
            CGEventField::ScrollWheelEventFixedPtDeltaAxis2,
            -f64::from(dx) / notch,
        );
        CGEvent::set_location(Some(&event), self.at);
        self.post(&event);
        Ok(())
    }

    fn key(&mut self, code: KeyCode, action: KeyAction) -> Result<()> {
        if let Some(nx) = nx_key(code) {
            return media(nx, action);
        }
        if code == KeyCode::CAPS_LOCK {
            // 按一下切换一次；松开没有作用。
            if action == KeyAction::Down {
                let caps = self.caps_lock()?;
                let on = caps.get()?;
                caps.set(!on)?;
            }
            return Ok(());
        }
        let keycode = mac_keycode(code).ok_or(Error::Unsupported("没有对应 Mac 按键的键"))?;
        let down = action != KeyAction::Up;
        let modifier = modifier_flag(keycode).is_some();
        if modifier {
            self.modifiers.retain(|&k| k != keycode);
            if down {
                self.modifiers.push(keycode);
            }
        }
        let event =
            CGEvent::new_keyboard_event(Some(&self.source.0), keycode, down).ok_or(Error::Os {
                call: "CGEventCreateKeyboardEvent",
                detail: "返回空".into(),
            })?;
        if modifier {
            // 真实键盘的修饰键只报告“修饰键变了”，标志位是变化之后的状态。
            CGEvent::set_type(Some(&event), CGEventType::FlagsChanged);
        } else if action == KeyAction::Repeat {
            CGEvent::set_integer_value_field(
                Some(&event),
                CGEventField::KeyboardEventAutorepeat,
                1,
            );
        }
        CGEvent::set_flags(Some(&event), self.flags() | key_flags(keycode));
        self.post(&event);
        Ok(())
    }

    /// 此刻应当带在事件上的修饰键标志：按着的修饰键，加上 Caps Lock。
    fn flags(&self) -> CGEventFlags {
        let mut bits = 0;
        for &keycode in &self.modifiers {
            bits |= modifier_flag(keycode).unwrap_or(0) | modifier_mask(keycode).bits();
        }
        let live = CGEventSource::flags_state(CGEventSourceStateID::CombinedSessionState);
        bits |= (live & CGEventFlags::MaskAlphaShift).bits();
        CGEventFlags::from_bits_retain(bits)
    }

    fn caps_lock(&mut self) -> Result<&CapsLock> {
        if self.caps_lock.is_none() {
            self.caps_lock = Some(CapsLock::open()?);
        }
        Ok(self.caps_lock.as_ref().expect("刚刚打开"))
    }
}

impl InputInject for MacInject {
    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        match *event {
            InputEvent::PointerMotion { position } => self.move_to(point(position)),
            InputEvent::PointerDelta { dx, dy } => {
                let to = CGPoint::new(self.at.x + f64::from(dx), self.at.y + f64::from(dy));
                self.move_to(to)
            }
            InputEvent::Button {
                button, pressed, ..
            } => self.button(button, pressed),
            InputEvent::Wheel { dx, dy, .. } => self.wheel(dx, dy),
            InputEvent::Key { code, action } => self.key(code, action),
        }
    }

    fn local_pointer(&mut self) -> Option<(Point, bool)> {
        // 一个空事件带着当前的光标位置（全局坐标，原点在主屏左上角，单位为点）。
        let at = CGEvent::location(CGEvent::new(None).as_deref());
        let held = CGEventSource::button_state(
            CGEventSourceStateID::CombinedSessionState,
            CGMouseButton::Left,
        );
        Some((Point::new(at.x.round() as i32, at.y.round() as i32), held))
    }

    fn sync_locks(&mut self, wanted: LockKeys, mask: LockKeys) -> Result<()> {
        // macOS 只有 Caps Lock。
        if !mask.contains(LockKeys::CAPS) {
            return Ok(());
        }
        let caps = self.caps_lock()?;
        let on = wanted.contains(LockKeys::CAPS);
        if caps.get()? != on {
            caps.set(on)?;
        }
        Ok(())
    }

    fn set_cursor_hidden(&mut self, hidden: bool) -> Result<()> {
        if hidden {
            // 藏的时候光标被挪到了中央；之后的点击、拖动从那里算起。
            let park = cursor::park_point();
            if self.cursor.hide(park, true) {
                self.at = point(park);
            }
        } else {
            self.cursor.show();
        }
        Ok(())
    }
}

fn point(p: Point) -> CGPoint {
    CGPoint::new(f64::from(p.x), f64::from(p.y))
}

/// 修饰键对应的“设备无关”标志位。
fn modifier_mask(keycode: u16) -> CGEventFlags {
    match keycode {
        0x38 | 0x3C => CGEventFlags::MaskShift,
        0x3B | 0x3E => CGEventFlags::MaskControl,
        0x3A | 0x3D => CGEventFlags::MaskAlternate,
        0x37 | 0x36 => CGEventFlags::MaskCommand,
        _ => CGEventFlags::empty(),
    }
}

/// 真实键盘在某些键上自带的标志：方向键带“小键盘”和 Fn，功能键和编辑键带 Fn，
/// 小键盘上的键带“小键盘”。有的程序靠它们认快捷键（例如 Control + 方向键切换桌面）。
fn key_flags(keycode: u16) -> CGEventFlags {
    match keycode {
        // ← → ↓ ↑
        0x7B..=0x7E => CGEventFlags::MaskNumericPad | CGEventFlags::MaskSecondaryFn,
        // F1–F20，Help、Home、Page Up、向前删除、End、Page Down
        0x40
        | 0x4F
        | 0x50
        | 0x5A
        | 0x60..=0x65
        | 0x67
        | 0x69..=0x6B
        | 0x6D
        | 0x6F
        | 0x71..=0x7A => CGEventFlags::MaskSecondaryFn,
        // 小键盘
        0x41 | 0x43 | 0x45 | 0x47 | 0x4B | 0x4C | 0x4E | 0x51..=0x59 | 0x5B | 0x5C | 0x5F => {
            CGEventFlags::MaskNumericPad
        }
        _ => CGEventFlags::empty(),
    }
}

/// 发出一个媒体键。键码和状态打包在 `data1` 里：高 16 位是键，接着 8 位是状态（0xA 按下、0xB 松开），
/// 最低位表示连发。
fn media(nx: u32, action: KeyAction) -> Result<()> {
    let state: isize = if action == KeyAction::Up { 0xB } else { 0xA };
    let repeat = isize::from(action == KeyAction::Repeat);
    let data1 = ((nx as isize) << 16) | (state << 8) | repeat;
    autoreleasepool(|_| {
        let event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            NSEventType::SystemDefined,
            CGPoint::new(0.0, 0.0),
            NSEventModifierFlags((state as usize) << 8),
            0.0,
            0,
            None,
            AUX_CONTROL_BUTTONS,
            data1,
            -1,
        )
        .and_then(|ns| ns.CGEvent())
        .ok_or(Error::Os {
            call: "NSEvent otherEventWithType",
            detail: "返回空".into(),
        })?;
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
        Ok(())
    })
}
