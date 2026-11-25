//! 输入事件：平台捕获产生、经网络传输、由平台注入的统一表示。

use crate::geometry::Point;
use crate::key::{KeyAction, KeyCode};
use crate::pointer::MouseButton;

/// 一个输入事件。坐标使用接收方设备的桌面坐标系。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputEvent {
    Key {
        code: KeyCode,
        action: KeyAction,
    },
    /// 光标移动到绝对位置。
    PointerMotion {
        position: Point,
    },
    /// 光标相对移动，用于游戏等锁定光标的程序。
    PointerDelta {
        dx: i32,
        dy: i32,
    },
    /// 鼠标按键。携带当时的光标位置，接收方先定位再执行。
    Button {
        button: MouseButton,
        pressed: bool,
        position: Point,
    },
    /// 滚轮，单位见 [`WHEEL_NOTCH`](crate::WHEEL_NOTCH)。`dy` 为正表示向上滚，
    /// `dx` 为正表示向右滚。携带当时的光标位置。
    Wheel {
        dx: i32,
        dy: i32,
        position: Point,
    },
}

/// 保留 Wayland 亚像素位移，返回可发送的整数增量。
#[derive(Clone, Copy, Debug, Default)]
pub struct RelativeMotionAccumulator {
    remainder: [f64; 2],
}

impl RelativeMotionAccumulator {
    pub fn update(&mut self, dx: f64, dy: f64) -> (i32, i32) {
        let mut delta = [0; 2];
        for (index, value) in [dx, dy].into_iter().enumerate() {
            let accumulated = self.remainder[index] + value;
            if accumulated.is_finite() {
                delta[index] = accumulated.trunc() as i32;
                self.remainder[index] = accumulated - f64::from(delta[index]);
            } else {
                self.remainder[index] = 0.0;
            }
        }
        (delta[0], delta[1])
    }
}
