//! 输入注入：虚拟指针和虚拟键盘协议，不需要 root 或 evdev 设备权限。

use std::collections::BTreeSet;
use std::os::fd::AsFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use ferry_platform::{Error, InputInject, Result};
use ferry_types::{InputEvent, KeyAction, KeyCode, LockKeys, Point, WHEEL_NOTCH};
use tracing::warn;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_output::{Mode, Transform, WlOutput};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

use crate::keymap::{
    current_keymap, hid_to_evdev, keymap_text, lock_keys_from_xkb, updated_keymap,
};
use crate::wayland::{
    STARTUP_TIMEOUT, Wake, connect, error, shared_file, timestamp, virtual_keyboard,
    virtual_pointer, wait,
};

enum Command {
    Event(InputEvent, mpsc::Sender<Result<()>>),
    Locks(LockKeys, LockKeys, mpsc::Sender<Result<()>>),
    End(mpsc::Sender<Result<()>>),
    Stop,
}

pub struct LinuxInject {
    commands: Option<mpsc::Sender<Command>>,
    wake: Option<Wake>,
    thread: Option<JoinHandle<()>>,
    alive: Arc<AtomicBool>,
}

impl LinuxInject {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let wake = Wake::new().map_err(|e| error("pipe2", e))?;
        let thread_wake = Wake::new().map_err(|e| error("pipe2", e))?;
        let startup_wake = thread_wake.clone();
        let alive = Arc::new(AtomicBool::new(true));
        let thread_alive = Arc::clone(&alive);
        let (ready_tx, ready_rx) = mpsc::channel();
        let startup_tx = tx.clone();
        let thread = std::thread::Builder::new()
            .name("ferry-wayland-inject".into())
            .spawn(move || {
                if let Err(e) = run(rx, thread_wake, &ready_tx) {
                    warn!("Wayland 输入注入已停止：{e}");
                    let _ = ready_tx.send(Err(e));
                }
                thread_alive.store(false, Ordering::SeqCst);
            })
            .map_err(|e| error("thread::spawn", e))?;
        match ready_rx.recv_timeout(STARTUP_TIMEOUT + Duration::from_secs(1)) {
            Ok(Ok(())) => Ok(Self {
                commands: Some(tx),
                wake: Some(wake),
                thread: Some(thread),
                alive,
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                alive.store(false, Ordering::SeqCst);
                let _ = startup_tx.send(Command::Stop);
                startup_wake.notify();
                Err(Error::Failed("Wayland 输入注入启动超时".into()))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
                Err(error("inject", "输入线程意外退出"))
            }
        }
    }

    fn call<T>(
        &self,
        make: impl FnOnce(mpsc::Sender<Result<()>>) -> Command,
        apply: impl FnOnce(Result<()>) -> T,
    ) -> Result<T> {
        let Some(tx) = &self.commands else {
            return Err(Error::Unsupported("输入注入尚未初始化"));
        };
        if !self.alive.load(Ordering::SeqCst) {
            return Err(error("inject", "Wayland 连接已断开"));
        }
        let (reply, response) = mpsc::channel();
        tx.send(make(reply))
            .map_err(|_| error("inject", "输入线程已退出"))?;
        self.wake.as_ref().map(Wake::notify);
        let result = response
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| error("inject", "输入线程没有响应"))?;
        Ok(apply(result))
    }
}

impl InputInject for LinuxInject {
    fn end_remote_input(&mut self) -> Result<()> {
        self.call(Command::End, |result| result)
            .and_then(|result| result)
    }

    fn check_available(&mut self) -> Result<()> {
        if self.alive.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(error("inject", "Wayland 连接已断开"))
        }
    }

    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        let event = *event;
        self.call(|reply| Command::Event(event, reply), |result| result)
            .and_then(|result| result)
    }

    fn sync_locks(&mut self, wanted: LockKeys, mask: LockKeys) -> Result<()> {
        self.call(|reply| Command::Locks(wanted, mask, reply), |result| result)
            .and_then(|result| result)
    }
}

impl Drop for LinuxInject {
    fn drop(&mut self) {
        if let Some(tx) = self.commands.take() {
            let _ = tx.send(Command::Stop);
        }
        self.wake.as_ref().map(Wake::notify);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct OutputInfo {
    _proxy: WlOutput,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: i32,
    rotated: bool,
}

impl OutputInfo {
    fn logical_size(&self) -> (i32, i32) {
        let (width, height) = if self.rotated {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        (
            (width / self.scale.max(1)).max(1),
            (height / self.scale.max(1)).max(1),
        )
    }
}

struct InjectState {
    qh: QueueHandle<InjectState>,
    seat: Option<WlSeat>,
    keyboard_observer: Option<WlKeyboard>,
    pointer_manager: Option<virtual_pointer::ZwlrVirtualPointerManagerV1>,
    pointer: Option<virtual_pointer::ZwlrVirtualPointerV1>,
    keyboard_manager: Option<virtual_keyboard::ZwpVirtualKeyboardManagerV1>,
    keyboard: Option<virtual_keyboard::ZwpVirtualKeyboardV1>,
    outputs: Vec<OutputInfo>,
    position: Point,
    pressed: BTreeSet<KeyCode>,
    locks: LockKeys,
    locks_known: bool,
    keymap_generation: u64,
}

impl InjectState {
    fn output_at(&self, position: Point) -> Option<&OutputInfo> {
        self.outputs.iter().min_by_key(|output| {
            let (width, height) = output.logical_size();
            let x = position.x.clamp(output.x, output.x + width - 1);
            let y = position.y.clamp(output.y, output.y + height - 1);
            (position.x - x).pow(2) + (position.y - y).pow(2)
        })
    }

    fn warp(&mut self, position: Point) -> Result<()> {
        let output = self
            .output_at(position)
            .ok_or_else(|| error("inject", "没有输出设备"))?;
        let (width, height) = output.logical_size();
        let x = (position.x - output.x).clamp(0, width - 1) as u32;
        let y = (position.y - output.y).clamp(0, height - 1) as u32;
        let pointer = self
            .pointer
            .as_ref()
            .ok_or_else(|| error("inject", "没有虚拟指针"))?;
        pointer.motion_absolute(timestamp(), x, y, width as u32, height as u32);
        pointer.frame();
        self.position = position;
        Ok(())
    }

    fn modifiers(&self) -> Result<()> {
        let mut mods = 0u32;
        for code in &self.pressed {
            mods |= match *code {
                KeyCode::LEFT_SHIFT | KeyCode::RIGHT_SHIFT => 1,
                KeyCode::LEFT_CTRL | KeyCode::RIGHT_CTRL => 4,
                KeyCode::LEFT_ALT | KeyCode::RIGHT_ALT => 8,
                KeyCode::LEFT_GUI | KeyCode::RIGHT_GUI => 64,
                _ => 0,
            };
        }
        let mut locked = 0;
        if self.locks.contains(LockKeys::CAPS) {
            locked |= 2;
        }
        if self.locks.contains(LockKeys::NUM) {
            locked |= 16;
        }
        if self.locks.contains(LockKeys::SCROLL) {
            locked |= 128;
        }
        self.keyboard
            .as_ref()
            .ok_or_else(|| error("inject", "没有虚拟键盘"))?
            .modifiers(mods, 0, locked, 0);
        Ok(())
    }

    fn key(&mut self, code: KeyCode, action: KeyAction) -> Result<()> {
        let Some(keycode) = hid_to_evdev(code) else {
            return Ok(());
        };
        let keyboard = self
            .keyboard
            .as_ref()
            .ok_or_else(|| error("inject", "没有虚拟键盘"))?;
        match action {
            KeyAction::Up => {
                self.pressed.remove(&code);
                keyboard.key(timestamp(), keycode + 8, 0);
            }
            KeyAction::Down | KeyAction::Repeat => {
                if matches!(
                    code,
                    KeyCode::CAPS_LOCK | KeyCode::NUM_LOCK | KeyCode::SCROLL_LOCK
                ) && action == KeyAction::Down
                {
                    if self.locks_known {
                        self.locks.toggle(match code {
                            KeyCode::CAPS_LOCK => LockKeys::CAPS,
                            KeyCode::NUM_LOCK => LockKeys::NUM,
                            _ => LockKeys::SCROLL,
                        });
                    }
                } else if !matches!(
                    code,
                    KeyCode::CAPS_LOCK | KeyCode::NUM_LOCK | KeyCode::SCROLL_LOCK
                ) {
                    self.pressed.insert(code);
                }
                keyboard.key(timestamp(), keycode + 8, 1);
                if matches!(
                    code,
                    KeyCode::CAPS_LOCK | KeyCode::NUM_LOCK | KeyCode::SCROLL_LOCK
                ) {
                    keyboard.key(timestamp(), keycode + 8, 0);
                }
            }
        }
        self.modifiers()
    }

    fn refresh_keymap(&mut self, conn: &Connection) -> Result<()> {
        let Some((generation, bytes)) = updated_keymap(self.keymap_generation) else {
            return Ok(());
        };
        let keyboard = self
            .keyboard
            .as_ref()
            .ok_or_else(|| error("inject", "没有虚拟键盘"))?;
        apply_keymap(keyboard, &bytes, conn)?;
        self.keymap_generation = generation;
        Ok(())
    }

    fn event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::PointerMotion { position } => self.warp(position),
            InputEvent::PointerDelta { dx, dy } => {
                let pointer = self
                    .pointer
                    .as_ref()
                    .ok_or_else(|| error("inject", "没有虚拟指针"))?;
                pointer.motion(timestamp(), f64::from(dx), f64::from(dy));
                pointer.frame();
                self.position = Point::new(
                    self.position.x.saturating_add(dx),
                    self.position.y.saturating_add(dy),
                );
                Ok(())
            }
            InputEvent::Button {
                button,
                pressed,
                position,
            } => {
                self.warp(position)?;
                let pointer = self
                    .pointer
                    .as_ref()
                    .ok_or_else(|| error("inject", "没有虚拟指针"))?;
                pointer.button(
                    timestamp(),
                    u32::from(button.number()) + 271,
                    if pressed { 1 } else { 0 },
                );
                pointer.frame();
                Ok(())
            }
            InputEvent::Wheel { dx, dy, position } => {
                self.warp(position)?;
                let pointer = self
                    .pointer
                    .as_ref()
                    .ok_or_else(|| error("inject", "没有虚拟指针"))?;
                pointer.axis_source(1);
                // wl_pointer.axis 里 0 是垂直滚动、1 是水平滚动，与直觉相反。
                for (axis, value) in [(1, dx), (0, dy)] {
                    if value == 0 {
                        continue;
                    }
                    let steps = f64::from(value) / f64::from(WHEEL_NOTCH);
                    pointer.axis(timestamp(), axis, steps);
                    pointer.axis_discrete(timestamp(), axis, steps, value / WHEEL_NOTCH);
                }
                pointer.frame();
                Ok(())
            }
            InputEvent::Key { code, action } => self.key(code, action),
        }
    }

    fn release_all(&mut self) -> Result<()> {
        if self.pressed.is_empty() {
            return Ok(());
        }
        let keyboard = self
            .keyboard
            .as_ref()
            .ok_or_else(|| error("inject", "没有虚拟键盘"))?;
        for code in std::mem::take(&mut self.pressed) {
            if let Some(keycode) = hid_to_evdev(code) {
                keyboard.key(timestamp(), keycode + 8, 0);
            }
        }
        self.modifiers()
    }
}

fn apply_keymap(
    keyboard: &virtual_keyboard::ZwpVirtualKeyboardV1,
    bytes: &[u8],
    conn: &Connection,
) -> Result<()> {
    let (file, path) = shared_file(bytes, "xkb")?;
    let result = {
        keyboard.keymap(1, file.as_fd(), bytes.len() as u32);
        conn.flush().map_err(|e| error("flush", e))
    };
    let _ = std::fs::remove_file(path);
    result
}

fn run(
    commands: mpsc::Receiver<Command>,
    wake: Wake,
    ready: &mpsc::Sender<Result<()>>,
) -> Result<()> {
    let conn = connect()?;
    let (globals, mut queue) =
        registry_queue_init::<InjectState>(&conn).map_err(|e| error("registry_queue_init", e))?;
    let qh = queue.handle();
    let mut state = InjectState {
        qh,
        seat: None,
        keyboard_observer: None,
        pointer_manager: None,
        pointer: None,
        keyboard_manager: None,
        keyboard: None,
        outputs: Vec::new(),
        position: Point::new(0, 0),
        pressed: BTreeSet::new(),
        locks: LockKeys::empty(),
        locks_known: false,
        keymap_generation: 0,
    };
    state.pointer_manager = globals.bind(&state.qh, 1..=1, ()).ok();
    state.keyboard_manager = globals.bind(&state.qh, 1..=1, ()).ok();
    for global in globals.contents().clone_list() {
        match global.interface.as_str() {
            "wl_seat" => {
                state.seat = Some(globals.registry().bind(
                    global.name,
                    global.version.min(9),
                    &state.qh,
                    (),
                ));
            }
            "wl_output" => {
                let index = state.outputs.len();
                let output =
                    globals
                        .registry()
                        .bind(global.name, global.version.min(4), &state.qh, index);
                state.outputs.push(OutputInfo {
                    _proxy: output,
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                    scale: 1,
                    rotated: false,
                });
            }
            _ => {}
        }
    }
    queue
        .roundtrip(&mut state)
        .map_err(|e| error("roundtrip", e))?;
    queue
        .roundtrip(&mut state)
        .map_err(|e| error("roundtrip", e))?;
    let seat = state
        .seat
        .as_ref()
        .ok_or(Error::Unsupported("Wayland 没有 seat"))?;
    if let Some(manager) = &state.pointer_manager {
        state.pointer = Some(virtual_pointer::create(manager, seat, &state.qh));
    }
    if let Some(manager) = &state.keyboard_manager {
        let keyboard = virtual_keyboard::create(manager, seat, &state.qh);
        let (generation, active) = current_keymap();
        let bytes = if active.is_empty() {
            keymap_text().as_bytes().to_vec()
        } else {
            active
        };
        apply_keymap(&keyboard, &bytes, &conn)?;
        state.keymap_generation = generation;
        state.keyboard = Some(keyboard);
    }
    if state.pointer.is_none() || state.keyboard.is_none() {
        return Err(Error::Unsupported("niri 没有虚拟指针或虚拟键盘协议"));
    }
    queue
        .roundtrip(&mut state)
        .map_err(|e| error("roundtrip", e))?;
    // 就绪表示初始化完成，不能等下面的常驻事件循环结束才通知。
    let _ = ready.send(Ok(()));
    loop {
        queue
            .dispatch_pending(&mut state)
            .map_err(|e| error("dispatch", e))?;
        conn.flush().map_err(|e| error("flush", e))?;
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Event(event, reply) => {
                    let result = state.refresh_keymap(&conn).and_then(|_| state.event(event));
                    let _ = reply.send(result);
                }
                Command::Locks(wanted, mask, reply) => {
                    if let Err(error) = state.refresh_keymap(&conn) {
                        let _ = reply.send(Err(error));
                        continue;
                    }
                    if !state.locks_known {
                        let _ = reply.send(Err(Error::Unsupported("无法读取本机锁定键的当前状态")));
                        continue;
                    }
                    let mut result = Ok(());
                    for (flag, code) in [
                        (LockKeys::CAPS, KeyCode::CAPS_LOCK),
                        (LockKeys::NUM, KeyCode::NUM_LOCK),
                        (LockKeys::SCROLL, KeyCode::SCROLL_LOCK),
                    ] {
                        if mask.contains(flag)
                            && state.locks.contains(flag) != wanted.contains(flag)
                        {
                            let keyboard = state.keyboard.as_ref().unwrap();
                            if let Some(keycode) = hid_to_evdev(code) {
                                keyboard.key(timestamp(), keycode + 8, 1);
                                keyboard.key(timestamp(), keycode + 8, 0);
                                state.locks.toggle(flag);
                                result = state.modifiers();
                            }
                        }
                    }
                    let _ = reply.send(result);
                }
                Command::End(reply) => {
                    let result = state
                        .refresh_keymap(&conn)
                        .and_then(|_| state.release_all());
                    let _ = reply.send(result);
                }
                Command::Stop => return Ok(()),
            }
        }
        wait(&conn, &wake).map_err(|e| error("poll", e))?;
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for InjectState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlOutput, usize> for InjectState {
    fn event(
        state: &mut Self,
        _output: &WlOutput,
        event: wayland_client::protocol::wl_output::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(output) = state.outputs.get_mut(*index) else {
            return;
        };
        match event {
            wayland_client::protocol::wl_output::Event::Geometry {
                x, y, transform, ..
            } => {
                output.x = x;
                output.y = y;
                output.rotated = matches!(
                    transform,
                    WEnum::Value(Transform::_90 | Transform::Flipped90 | Transform::Flipped270)
                );
            }
            wayland_client::protocol::wl_output::Event::Mode {
                flags: WEnum::Value(flags),
                width,
                height,
                ..
            } if flags.contains(Mode::Current) => {
                output.width = width;
                output.height = height;
            }
            wayland_client::protocol::wl_output::Event::Scale { factor } => output.scale = factor,
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for InjectState {
    fn event(
        state: &mut Self,
        seat: &WlSeat,
        event: wayland_client::protocol::wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_seat::Event::Capabilities { capabilities } = event
            && let WEnum::Value(caps) = capabilities
            && caps.contains(wayland_client::protocol::wl_seat::Capability::Keyboard)
            && state.keyboard_observer.is_none()
        {
            state.keyboard_observer = Some(seat.get_keyboard(qh, ()));
        }
    }
}

impl Dispatch<WlKeyboard, ()> for InjectState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wayland_client::protocol::wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_keyboard::Event::Modifiers { mods_locked, .. } = event {
            state.locks = lock_keys_from_xkb(mods_locked);
            state.locks_known = true;
        }
    }
}

impl Dispatch<virtual_pointer::ZwlrVirtualPointerManagerV1, ()> for InjectState {
    fn event(
        _: &mut Self,
        _: &virtual_pointer::ZwlrVirtualPointerManagerV1,
        _: virtual_pointer::zwlr_virtual_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<virtual_pointer::ZwlrVirtualPointerV1, ()> for InjectState {
    fn event(
        _: &mut Self,
        _: &virtual_pointer::ZwlrVirtualPointerV1,
        _: virtual_pointer::zwlr_virtual_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<virtual_keyboard::ZwpVirtualKeyboardManagerV1, ()> for InjectState {
    fn event(
        _: &mut Self,
        _: &virtual_keyboard::ZwpVirtualKeyboardManagerV1,
        _: virtual_keyboard::zwp_virtual_keyboard_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<virtual_keyboard::ZwpVirtualKeyboardV1, ()> for InjectState {
    fn event(
        _: &mut Self,
        _: &virtual_keyboard::ZwpVirtualKeyboardV1,
        _: virtual_keyboard::zwp_virtual_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
