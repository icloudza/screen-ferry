//! 输入捕获：定义 `InputCapture` trait。

use ferry_types::{KeyAction, KeyCode, LockKeys, MouseButton, Point};

use crate::error::Result;

/// 捕获到的一个本机输入。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureEvent {
    /// 鼠标移动：本机光标位置，以及本次的相对位移。抓取期间光标被固定，只有位移有意义。
    Pointer {
        position: Point,
        dx: i32,
        dy: i32,
    },
    Button {
        button: MouseButton,
        pressed: bool,
    },
    /// 滚轮，单位与方向见 [`ferry_types::InputEvent::Wheel`]。
    Wheel {
        dx: i32,
        dy: i32,
    },
    Key {
        code: KeyCode,
        action: KeyAction,
    },
    /// 抓取期间本机键盘焦点或指针锁定意外丢失，必须把控制权交还本机。
    FocusLost,
}

/// 交给 sink 的一个捕获事件，带着它发生时是否处于抓取状态。
///
/// 事件从捕获线程送到处理方要花一点时间，切换屏幕的那一刻总有几个事件还在路上。
/// 处理方靠 `grabbed` 判断它当时是交给了本机还是被吞掉了，和现在的状态对不上就是过时的。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Captured {
    pub event: CaptureEvent,
    /// 事件发生时是否处于抓取状态：为真时本机没有收到它，光标也没有动。
    pub grabbed: bool,
}

/// 接收捕获事件的回调。在捕获线程上调用，必须立即返回。
pub type CaptureSink = Box<dyn FnMut(Captured) + Send>;

/// 本机键鼠捕获。
pub trait InputCapture: Send {
    /// 开始捕获，事件交给 `sink`，直到本对象被丢弃。
    fn start(&mut self, sink: CaptureSink) -> Result<()>;

    /// 抓取：本机不再响应键鼠，光标固定在原地，事件只交给 `sink`。
    ///
    /// 抓取前就已按下的键，之后的连发和松开仍交给本机，不交给 `sink`，
    /// 以免本机以为它一直按着。
    fn grab(&self) -> Result<()>;

    /// 结束抓取，并把本机光标放到 `at`。
    ///
    /// 必须先放好光标再结束抓取：标记为未抓取的事件，位置都应当是放好之后的。
    fn release(&self, at: Point) -> Result<()>;

    /// 本机当前的锁定键状态。平台没有的锁定键视为关闭。
    fn lock_keys(&self) -> LockKeys;
}
