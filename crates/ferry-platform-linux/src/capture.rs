//! 输入捕获：透明 layer-shell 表面接收键盘，指针锁定时转发相对位移。

use std::collections::BTreeSet;
use std::os::fd::AsFd;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ferry_platform::{CaptureEvent, CaptureSink, Captured, Error, InputCapture, Result};
use ferry_types::{KeyAction, KeyCode, LockKeys, Point, RelativeMotionAccumulator, WHEEL_NOTCH};
use tracing::{debug, warn};
use wayland_client::Connection;
use wayland_client::Dispatch;
use wayland_client::Proxy;
use wayland_client::QueueHandle;
use wayland_client::WEnum;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_keyboard::{KeyState, WlKeyboard};
use wayland_client::protocol::wl_output::{Mode, Transform, WlOutput};
use wayland_client::protocol::wl_pointer::{Axis, ButtonState, Event as PointerEvent, WlPointer};
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::{Capability, WlSeat};
use wayland_client::protocol::wl_shm::Format;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1;
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibitor_v1::{
    Event as ShortcutsInhibitorEvent, ZwpKeyboardShortcutsInhibitorV1,
};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::ZwpLockedPointerV1;
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::{
    Lifetime, ZwpPointerConstraintsV1,
};
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1;
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1::{
    Event as RelativePointerEvent, ZwpRelativePointerV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer as WlrLayer, ZwlrLayerShellV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1,
};

use crate::keymap::{lock_keys_from_xkb, read_keymap, remember_keymap, wayland_key_to_hid};
use crate::wayland::{
    STARTUP_TIMEOUT, Wake, connect, error, shared_file, timestamp, virtual_pointer, wait,
};

#[derive(Debug)]
enum Command {
    Grab(mpsc::Sender<Result<()>>),
    Release(Point, mpsc::Sender<Result<()>>),
    Stop,
}

pub struct LinuxCapture {
    commands: Option<mpsc::Sender<Command>>,
    wake: Option<Wake>,
    thread: Option<JoinHandle<()>>,
    locks: Arc<std::sync::Mutex<LockKeys>>,
}

impl LinuxCapture {
    pub fn new() -> Self {
        Self {
            commands: None,
            wake: None,
            thread: None,
            locks: Arc::new(std::sync::Mutex::new(LockKeys::empty())),
        }
    }
}

impl Default for LinuxCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl InputCapture for LinuxCapture {
    fn start(&mut self, sink: CaptureSink) -> Result<()> {
        if self.commands.is_some() {
            return Err(Error::Unsupported("已经在捕获"));
        }
        let (tx, rx) = mpsc::channel();
        let wake = Wake::new().map_err(|e| error("pipe2", e))?;
        let thread_wake = Wake::new().map_err(|e| error("pipe2", e))?;
        let startup_wake = thread_wake.clone();
        let startup_tx = tx.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let locks = Arc::clone(&self.locks);
        let thread = std::thread::Builder::new()
            .name("ferry-wayland-capture".into())
            .spawn(move || {
                if let Err(e) = run(rx, thread_wake, sink, locks, &ready_tx) {
                    warn!("Wayland 输入捕获已停止：{e}");
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| error("thread::spawn", e))?;
        match ready_rx.recv_timeout(STARTUP_TIMEOUT + Duration::from_secs(1)) {
            Ok(Ok(())) => {
                self.commands = Some(tx);
                self.wake = Some(wake);
                self.thread = Some(thread);
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = startup_tx.send(Command::Stop);
                startup_wake.notify();
                Err(Error::Failed("Wayland 捕获启动超时".into()))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
                Err(error("capture", "捕获线程意外退出"))
            }
        }
    }

    fn grab(&self) -> Result<()> {
        self.command(GrabCommand::Grab)
    }

    fn release(&self, at: Point) -> Result<()> {
        self.command(GrabCommand::Release(at))
    }

    fn lock_keys(&self) -> LockKeys {
        self.locks.lock().map_or(LockKeys::empty(), |locks| *locks)
    }
}

enum GrabCommand {
    Grab,
    Release(Point),
}

impl LinuxCapture {
    fn command(&self, command: GrabCommand) -> Result<()> {
        let Some(tx) = &self.commands else {
            return Err(Error::Unsupported("捕获尚未开始"));
        };
        let (reply, response) = mpsc::channel();
        let command = match command {
            GrabCommand::Grab => Command::Grab(reply),
            GrabCommand::Release(at) => Command::Release(at, reply),
        };
        tx.send(command)
            .map_err(|_| error("capture", "捕获线程已退出"))?;
        self.wake.as_ref().map(Wake::notify);
        response
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| error("capture", "捕获线程没有响应"))?
    }
}

impl Drop for LinuxCapture {
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
    proxy: WlOutput,
    name: String,
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

struct CaptureLayer {
    proxy: ZwlrLayerSurfaceV1,
    surface: WlSurface,
    edge_region: WlRegion,
    full_region: WlRegion,
    buffer: Option<WlBuffer>,
    output: usize,
    width: u32,
    height: u32,
}

struct CaptureState {
    qh: QueueHandle<CaptureState>,
    compositor: Option<wayland_client::protocol::wl_compositor::WlCompositor>,
    shm: Option<WlShm>,
    layer_shell: Option<ZwlrLayerShellV1>,
    seat: Option<WlSeat>,
    pointer: Option<WlPointer>,
    keyboard: Option<WlKeyboard>,
    keyboard_surface: Option<usize>,
    constraints: Option<ZwpPointerConstraintsV1>,
    shortcuts_inhibit_manager: Option<ZwpKeyboardShortcutsInhibitManagerV1>,
    shortcuts_inhibitor: Option<ZwpKeyboardShortcutsInhibitorV1>,
    relative_manager: Option<ZwpRelativePointerManagerV1>,
    virtual_pointer_manager: Option<virtual_pointer::ZwlrVirtualPointerManagerV1>,
    virtual_pointer: Option<virtual_pointer::ZwlrVirtualPointerV1>,
    pending_shared: Vec<(std::fs::File, std::path::PathBuf)>,
    relative: Option<ZwpRelativePointerV1>,
    shared_locks: Arc<std::sync::Mutex<LockKeys>>,
    outputs: Vec<OutputInfo>,
    layers: Vec<CaptureLayer>,
    active: usize,
    position: Point,
    grab_origin: Point,
    sink: Option<CaptureSink>,
    grabbed: bool,
    pointer_inside: bool,
    pressed: BTreeSet<KeyCode>,
    local_keys: BTreeSet<KeyCode>,
    locks: LockKeys,
    locked: Option<ZwpLockedPointerV1>,
    grab_reply: Option<mpsc::Sender<Result<()>>>,
    grab_started: Option<Instant>,
    grab_pointer_locked: bool,
    grab_keyboard_focused: bool,
    grab_shortcuts_inhibited: bool,
    axis: [f64; 2],
    axis120: [i32; 2],
    axis_discrete: [i32; 2],
    relative_motion: RelativeMotionAccumulator,
}

impl CaptureState {
    fn emit(&mut self, event: CaptureEvent, grabbed: bool) {
        if let Some(sink) = self.sink.as_mut() {
            sink(Captured { event, grabbed });
        }
    }

    fn output(&self, index: usize) -> Option<&OutputInfo> {
        self.outputs.get(index)
    }

    fn lock_active(&mut self) -> Result<()> {
        self.grab_origin = self.position;
        self.local_keys = self.pressed.clone();
        self.keyboard_surface = None;
        self.grab_pointer_locked = false;
        self.grab_keyboard_focused = false;
        self.grab_shortcuts_inhibited = false;
        for layer in &self.layers {
            let active = layer.output == self.active;
            layer
                .proxy
                .set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.surface.set_input_region(Some(if active {
                &layer.full_region
            } else {
                &layer.edge_region
            }));
            layer.surface.commit();
        }
        let layer = &self.layers[self.active].surface;
        let pointer = self
            .pointer
            .as_ref()
            .ok_or_else(|| error("capture", " seat 没有指针"))?;
        let constraints = self
            .constraints
            .as_ref()
            .ok_or_else(|| error("capture", "niri 没有指针锁定协议"))?;
        self.locked = Some(constraints.lock_pointer(
            layer,
            pointer,
            None,
            Lifetime::Persistent,
            &self.qh,
            (),
        ));
        self.grab_started = Some(Instant::now());
        debug!(output = self.active, "开始交接键盘焦点与指针锁定");
        Ok(())
    }

    fn request_keyboard_focus(&mut self) {
        if self.grab_reply.is_none() {
            return;
        }
        let Some(surface) = self
            .layers
            .get(self.active)
            .map(|layer| layer.surface.clone())
        else {
            return;
        };
        if let (Some(manager), Some(seat)) = (&self.shortcuts_inhibit_manager, &self.seat)
            && self.shortcuts_inhibitor.is_none()
        {
            self.shortcuts_inhibitor =
                Some(manager.inhibit_shortcuts(&surface, seat, &self.qh, ()));
            self.grab_shortcuts_inhibited = false;
            debug!(output = self.active, "已请求 niri 抑制本机快捷键");
        }
        self.layers[self.active]
            .proxy
            .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        self.layers[self.active].surface.commit();
        debug!(output = self.active, "指针已锁定，请求捕获层键盘焦点");
    }

    fn finish_grab_if_ready(&mut self) {
        let shortcuts_ready =
            self.shortcuts_inhibit_manager.is_none() || self.grab_shortcuts_inhibited;
        if self.grab_reply.is_none()
            || !self.grab_pointer_locked
            || !self.grab_keyboard_focused
            || !shortcuts_ready
        {
            return;
        }
        let output = &self.outputs[self.active];
        let (width, height) = output.logical_size();
        self.position = Point::new(output.x + (width / 2), output.y + (height / 2));
        self.grabbed = true;
        self.grab_started = None;
        debug!(output = self.active, shortcuts_ready, "键盘独占交接完成");
        if let Some(reply) = self.grab_reply.take() {
            let _ = reply.send(Ok(()));
        }
    }

    fn reset_grab(&mut self) {
        self.grabbed = false;
        self.grab_started = None;
        self.grab_pointer_locked = false;
        self.grab_keyboard_focused = false;
        self.grab_shortcuts_inhibited = false;
        self.keyboard_surface = None;
        if let Some(locked) = self.locked.take() {
            locked.destroy();
        }
        if let Some(inhibitor) = self.shortcuts_inhibitor.take() {
            inhibitor.destroy();
        }
        self.local_keys.clear();
        self.pressed.clear();
        self.pointer_inside = false;
        self.axis = [0.0; 2];
        self.axis120 = [0; 2];
        self.axis_discrete = [0; 2];
        for layer in &self.layers {
            layer
                .proxy
                .set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.surface.set_input_region(Some(&layer.edge_region));
            layer.surface.commit();
        }
    }

    fn fail_grab(&mut self, reason: &'static str) {
        let _ = self.unlock_active(self.grab_origin);
        if let Some(reply) = self.grab_reply.take() {
            let _ = reply.send(Err(Error::Unsupported(reason)));
        }
    }

    fn focus_lost(&mut self, reason: &'static str) {
        if !self.grabbed {
            return;
        }
        warn!("{reason}，立即交还本机控制");
        self.position = self.grab_origin;
        self.reset_grab();
        self.emit(CaptureEvent::FocusLost, true);
    }

    fn unlock_active(&mut self, at: Point) -> Result<()> {
        self.reset_grab();
        let output = self
            .output(self.active)
            .ok_or_else(|| error("capture", "没有活动的显示器"))?;
        let (width, height) = output.logical_size();
        let x = (at.x - output.x).clamp(0, width - 1);
        let y = (at.y - output.y).clamp(0, height - 1);
        let width = width as u32;
        let height = height as u32;
        if let (Some(manager), Some(seat)) = (&self.virtual_pointer_manager, &self.seat) {
            let pointer = self
                .virtual_pointer
                .get_or_insert_with(|| virtual_pointer::create(manager, seat, &self.qh));
            pointer.motion_absolute(timestamp(), x as u32, y as u32, width, height);
            pointer.frame();
        }
        self.position = at;
        Ok(())
    }

    fn update_regions(&mut self) {
        let Some(compositor) = &self.compositor else {
            return;
        };
        for index in 0..self.layers.len() {
            // 尚未 configure 的 layer 没有缓冲区，不能在这里提前提交。
            if self.layers[index].buffer.is_none() {
                continue;
            }
            let output_index = self.layers[index].output;
            let output_size = self
                .output(output_index)
                .map(OutputInfo::logical_size)
                .unwrap_or((1, 1));
            let layer = &mut self.layers[index];
            let (width, height) = if layer.buffer.is_some() {
                (layer.width as i32, layer.height as i32)
            } else {
                output_size
            };
            let edge_region = compositor.create_region(&self.qh, ());
            edge_region.add(0, 0, 1, height);
            edge_region.add(width - 1, 0, 1, height);
            edge_region.add(0, 0, width, 1);
            edge_region.add(0, height - 1, width, 1);
            let full_region = compositor.create_region(&self.qh, ());
            full_region.add(0, 0, width, height);
            layer.surface.set_input_region(Some(
                if layer.output == self.active && (self.grabbed || self.grab_reply.is_some()) {
                    &full_region
                } else {
                    &edge_region
                },
            ));
            layer.surface.commit();
            layer.edge_region.destroy();
            layer.full_region.destroy();
            layer.edge_region = edge_region;
            layer.full_region = full_region;
        }
    }
}

fn run(
    commands: mpsc::Receiver<Command>,
    wake: Wake,
    sink: CaptureSink,
    locks: Arc<std::sync::Mutex<LockKeys>>,
    ready: &mpsc::Sender<Result<()>>,
) -> Result<()> {
    let conn = connect()?;
    let (globals, mut queue) =
        registry_queue_init::<CaptureState>(&conn).map_err(|e| error("registry_queue_init", e))?;
    let qh = queue.handle();
    let mut state = CaptureState {
        qh,
        compositor: None,
        shm: None,
        layer_shell: None,
        seat: None,
        pointer: None,
        keyboard: None,
        keyboard_surface: None,
        constraints: None,
        shortcuts_inhibit_manager: None,
        shortcuts_inhibitor: None,
        relative_manager: None,
        virtual_pointer_manager: None,
        virtual_pointer: None,
        pending_shared: Vec::new(),
        relative: None,
        shared_locks: locks,
        outputs: Vec::new(),
        layers: Vec::new(),
        active: 0,
        position: Point::new(0, 0),
        grab_origin: Point::new(0, 0),
        sink: Some(sink),
        grabbed: false,
        pointer_inside: false,
        pressed: BTreeSet::new(),
        local_keys: BTreeSet::new(),
        locks: LockKeys::empty(),
        locked: None,
        grab_reply: None,
        grab_started: None,
        grab_pointer_locked: false,
        grab_keyboard_focused: false,
        grab_shortcuts_inhibited: false,
        axis: [0.0; 2],
        axis120: [0; 2],
        axis_discrete: [0; 2],
        relative_motion: RelativeMotionAccumulator::default(),
    };
    state.compositor = globals.bind(&state.qh, 1..=6, ()).ok();
    state.shm = globals.bind(&state.qh, 1..=1, ()).ok();
    state.layer_shell = globals.bind(&state.qh, 1..=4, ()).ok();
    state.constraints = globals.bind(&state.qh, 1..=1, ()).ok();
    state.shortcuts_inhibit_manager = globals.bind(&state.qh, 1..=1, ()).ok();
    state.relative_manager = globals.bind(&state.qh, 1..=1, ()).ok();
    state.virtual_pointer_manager = globals.bind(&state.qh, 1..=1, ()).ok();
    for global in globals.contents().clone_list() {
        match global.interface.as_str() {
            "wl_seat" => {
                state.seat = Some(globals.registry().bind(
                    global.name,
                    global.version.min(7),
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
                    proxy: output,
                    name: String::new(),
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
    if state.compositor.is_none()
        || state.shm.is_none()
        || state.layer_shell.is_none()
        || state.outputs.is_empty()
    {
        return Err(Error::Unsupported(
            "Wayland 缺少 layer-shell、wl_shm 或输出设备",
        ));
    }
    if state.pointer.is_none()
        || state.keyboard.is_none()
        || state.constraints.is_none()
        || state.relative_manager.is_none()
    {
        return Err(Error::Unsupported("niri 缺少输入捕获所需协议"));
    }
    if state.shortcuts_inhibit_manager.is_none() {
        warn!("Wayland 缺少 keyboard-shortcuts-inhibit，无法阻止本机全局快捷键");
    }
    for (index, output) in state.outputs.iter().enumerate() {
        let surface = state
            .compositor
            .as_ref()
            .unwrap()
            .create_surface(&state.qh, ());
        let layer = state.layer_shell.as_ref().unwrap().get_layer_surface(
            &surface,
            Some(&output.proxy),
            WlrLayer::Overlay,
            "screenferry-capture".to_string(),
            &state.qh,
            index,
        );
        layer.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        let (width, height) = output.logical_size();
        let compositor = state.compositor.as_ref().unwrap();
        let edge_region = compositor.create_region(&state.qh, ());
        edge_region.add(0, 0, 1, height);
        edge_region.add(width - 1, 0, 1, height);
        edge_region.add(0, 0, width, 1);
        edge_region.add(0, height - 1, width, 1);
        let full_region = compositor.create_region(&state.qh, ());
        full_region.add(0, 0, width, height);
        surface.set_input_region(Some(&edge_region));
        surface.commit();
        state.layers.push(CaptureLayer {
            proxy: layer,
            surface,
            edge_region,
            full_region,
            buffer: None,
            output: index,
            width: 0,
            height: 0,
        });
    }
    queue
        .roundtrip(&mut state)
        .map_err(|e| error("roundtrip", e))?;
    if state.relative_manager.is_some() && state.pointer.is_some() {
        let relative = state
            .relative_manager
            .as_ref()
            .unwrap()
            .get_relative_pointer(state.pointer.as_ref().unwrap(), &state.qh, ());
        state.relative = Some(relative);
    }
    // 就绪表示初始化完成，不能等下面的常驻事件循环结束才通知。
    let _ = ready.send(Ok(()));
    loop {
        queue
            .dispatch_pending(&mut state)
            .map_err(|e| error("dispatch", e))?;
        conn.flush().map_err(|e| error("flush", e))?;
        for (_, path) in state.pending_shared.drain(..) {
            let _ = std::fs::remove_file(path);
        }
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Grab(reply) => {
                    let result = state.lock_active();
                    if let Err(e) = result {
                        let _ = reply.send(Err(e));
                    } else {
                        state.grab_reply = Some(reply);
                    }
                }
                Command::Release(at, reply) => {
                    let result = state.unlock_active(at);
                    let _ = reply.send(result);
                }
                Command::Stop => return Ok(()),
            }
        }
        if let Some(started) = state.grab_started
            && started.elapsed() > Duration::from_secs(2)
        {
            state.fail_grab("niri 没有完成指针锁定、键盘焦点和快捷键抑制交接");
        }
        wait(&conn, &wake).map_err(|e| error("poll", e))?;
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for CaptureState {
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

impl Dispatch<WlSeat, ()> for CaptureState {
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
        {
            if caps.contains(Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
            if caps.contains(Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
        }
    }
}

impl Dispatch<WlOutput, usize> for CaptureState {
    fn event(
        state: &mut Self,
        _output: &WlOutput,
        event: wayland_client::protocol::wl_output::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(info) = state.outputs.get_mut(*index) else {
            return;
        };
        match event {
            wayland_client::protocol::wl_output::Event::Geometry {
                x, y, transform, ..
            } => {
                info.x = x;
                info.y = y;
                info.rotated = matches!(
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
                info.width = width;
                info.height = height;
            }
            wayland_client::protocol::wl_output::Event::Scale { factor } => info.scale = factor,
            wayland_client::protocol::wl_output::Event::Name { name } => info.name = name,
            wayland_client::protocol::wl_output::Event::Done => state.update_regions(),
            _ => {}
        }
    }
}

impl Dispatch<WlSurface, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: wayland_client::protocol::wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlRegion, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlRegion,
        _: wayland_client::protocol::wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, usize> for CaptureState {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                surface.ack_configure(serial);
                let output_size = state
                    .layers
                    .get(*index)
                    .and_then(|layer| state.output(layer.output))
                    .map(OutputInfo::logical_size)
                    .unwrap_or((1, 1));
                let Some(layer) = state.layers.get_mut(*index) else {
                    return;
                };
                if layer.buffer.is_some() && layer.width == width && layer.height == height {
                    layer.surface.commit();
                    return;
                }
                let (width, height) = if width == 0 || height == 0 {
                    output_size
                } else {
                    (width as i32, height as i32)
                };
                let width = width.max(1) as u32;
                let height = height.max(1) as u32;
                let Some(byte_len) = width
                    .checked_mul(height)
                    .and_then(|pixels| pixels.checked_mul(4))
                    .filter(|&size| size <= 256 * 1024 * 1024)
                else {
                    warn!(width, height, "捕获层尺寸无效");
                    surface.destroy();
                    return;
                };
                let (file, path) = match shared_file(&vec![0; byte_len as usize], "shm") {
                    Ok(shared) => shared,
                    Err(error) => {
                        warn!("创建捕获层共享缓冲失败：{error}");
                        surface.destroy();
                        return;
                    }
                };
                let shm = state.shm.as_ref().unwrap();
                let pool = shm.create_pool(file.as_fd(), byte_len as i32, &state.qh, ());
                let buffer = pool.create_buffer(
                    0,
                    width as i32,
                    height as i32,
                    width as i32 * 4,
                    Format::Argb8888,
                    &state.qh,
                    (),
                );
                pool.destroy();
                layer.surface.attach(Some(&buffer), 0, 0);
                layer.surface.damage(0, 0, width as i32, height as i32);
                if let Some(previous) = layer.buffer.replace(buffer) {
                    previous.destroy();
                }
                layer.width = width;
                layer.height = height;
                state.pending_shared.push((file, path));
                state.update_regions();
            }
            wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Closed => surface.destroy(),
            _ => {}
        }
    }
}

impl Dispatch<WlPointer, ()> for CaptureState {
    fn event(
        state: &mut Self,
        pointer: &WlPointer,
        event: PointerEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let _ = pointer;
        match event {
            PointerEvent::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                if let Some(index) = state
                    .layers
                    .iter()
                    .position(|layer| layer.surface.id() == surface.id())
                {
                    state.active = index;
                    state.pointer_inside = true;
                    let output = &state.outputs[index];
                    state.position = Point::new(
                        output.x + surface_x.round() as i32,
                        output.y + surface_y.round() as i32,
                    );
                    debug!(
                        output = state.active,
                        surface_x,
                        surface_y,
                        position = ?state.position,
                        "捕获层收到指针进入"
                    );
                }
            }
            PointerEvent::Leave { surface, .. } => {
                if state
                    .layers
                    .iter()
                    .any(|layer| layer.surface.id() == surface.id())
                {
                    state.pointer_inside = false;
                    debug!("捕获层收到指针离开");
                }
            }
            PointerEvent::Motion {
                time,
                surface_x,
                surface_y,
                ..
            } => {
                if !state.grabbed {
                    let output = &state.outputs[state.active];
                    state.position = Point::new(
                        output.x + surface_x.round() as i32,
                        output.y + surface_y.round() as i32,
                    );
                }
                let _ = time;
            }
            PointerEvent::Button {
                button,
                state: button_state,
                ..
            } => {
                if !state.grabbed {
                    return;
                }
                let WEnum::Value(button_state) = button_state else {
                    return;
                };
                if button < 272 {
                    return;
                }
                let number = match button {
                    272 => 1,
                    273 => 2,
                    274 => 3,
                    other => other.saturating_sub(271).min(u8::MAX as u32) as u8,
                };
                if let Some(button) = ferry_types::MouseButton::new(number) {
                    state.emit(
                        CaptureEvent::Button {
                            button,
                            pressed: button_state == ButtonState::Pressed,
                        },
                        true,
                    );
                }
            }
            PointerEvent::Axis { axis, value, .. } => {
                if state.grabbed
                    && let WEnum::Value(axis) = axis
                {
                    state.axis[axis_index(axis)] += value;
                }
            }
            PointerEvent::AxisValue120 { axis, value120 } => {
                if state.grabbed
                    && let WEnum::Value(axis) = axis
                {
                    state.axis120[axis_index(axis)] =
                        state.axis120[axis_index(axis)].saturating_add(value120);
                }
            }
            PointerEvent::AxisDiscrete { axis, discrete } => {
                if state.grabbed
                    && let WEnum::Value(axis) = axis
                {
                    state.axis_discrete[axis_index(axis)] =
                        state.axis_discrete[axis_index(axis)].saturating_add(discrete);
                }
            }
            PointerEvent::Frame => {
                if !state.grabbed {
                    state.axis = [0.0; 2];
                    state.axis120 = [0; 2];
                    state.axis_discrete = [0; 2];
                    return;
                }
                let mut values = [0; 2];
                for (index, value) in values.iter_mut().enumerate() {
                    if state.axis_discrete[index] != 0 {
                        *value = state.axis_discrete[index];
                        state.axis_discrete[index] = 0;
                        state.axis120[index] = 0;
                        state.axis[index] = 0.0;
                    } else if state.axis120[index] != 0 {
                        *value = state.axis120[index] / WHEEL_NOTCH;
                        state.axis120[index] %= WHEEL_NOTCH;
                        state.axis[index] = 0.0;
                    } else {
                        let steps = (state.axis[index] / f64::from(WHEEL_NOTCH)).trunc() as i32;
                        *value = steps;
                        state.axis[index] -= f64::from(steps * WHEEL_NOTCH);
                    }
                }
                if values[0] != 0 || values[1] != 0 {
                    state.emit(
                        CaptureEvent::Wheel {
                            dx: values[0].saturating_mul(WHEEL_NOTCH),
                            dy: values[1].saturating_mul(WHEEL_NOTCH),
                        },
                        true,
                    );
                }
            }
            _ => {}
        }
    }
}

fn axis_index(axis: Axis) -> usize {
    if axis == Axis::VerticalScroll { 1 } else { 0 }
}

impl Dispatch<WlKeyboard, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wayland_client::protocol::wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_keyboard::Event::Enter { surface, keys, .. } => {
                let index = state
                    .layers
                    .iter()
                    .position(|layer| layer.surface.id() == surface.id());
                let entered: BTreeSet<KeyCode> = keys
                    .into_iter()
                    .filter_map(|key| wayland_key_to_hid(u32::from(key)))
                    .collect();
                if state.grab_reply.is_some() {
                    state.pressed = entered;
                    state.local_keys = state.pressed.clone();
                } else {
                    state.pressed.extend(entered);
                }
                state.keyboard_surface = index;
                debug!(?index, active = state.active, "捕获层键盘焦点进入");
                if state.grab_reply.is_some() {
                    if index == Some(state.active) && state.grab_pointer_locked {
                        state.grab_keyboard_focused = true;
                        state.finish_grab_if_ready();
                    }
                } else if state.grabbed && index != Some(state.active) {
                    state.focus_lost("本机键盘转到了其他窗口");
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Leave { surface, .. } => {
                let index = state
                    .layers
                    .iter()
                    .position(|layer| layer.surface.id() == surface.id());
                if state.keyboard_surface == index {
                    state.keyboard_surface = None;
                }
                debug!(
                    ?index,
                    active = state.active,
                    grabbed = state.grabbed,
                    "捕获层键盘焦点离开"
                );
                if index == Some(state.active) {
                    if state.grabbed {
                        state.focus_lost("本机键盘焦点丢失");
                    } else if state.grab_reply.is_some() {
                        state.grab_keyboard_focused = false;
                    }
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Key {
                key,
                state: key_state,
                ..
            } => {
                let WEnum::Value(key_state) = key_state else {
                    return;
                };
                let Some(code) = wayland_key_to_hid(key) else {
                    debug!(raw_key = key, "忽略没有 HID 对应的 Wayland 键码");
                    return;
                };
                debug!(
                    raw_key = key,
                    ?code,
                    hid = code.raw(),
                    grabbed = state.grabbed,
                    keyboard_focus = ?state.keyboard_surface,
                    "收到 Wayland 键盘事件"
                );
                let action = if key_state == KeyState::Released {
                    let was_pressed = state.pressed.remove(&code);
                    let was_local = state.local_keys.remove(&code);
                    if state.grabbed && (!was_pressed || was_local) {
                        return;
                    }
                    KeyAction::Up
                } else if state.pressed.contains(&code) {
                    KeyAction::Repeat
                } else {
                    state.pressed.insert(code);
                    KeyAction::Down
                };
                if state.grabbed && !state.local_keys.contains(&code) {
                    state.emit(CaptureEvent::Key { code, action }, true);
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Keymap { format, fd, size } => {
                if format
                    == WEnum::Value(wayland_client::protocol::wl_keyboard::KeymapFormat::XkbV1)
                {
                    match read_keymap(fd, size) {
                        Ok(bytes) => remember_keymap(bytes),
                        Err(error) => eprintln!("读取 XKB keymap 失败：{error}"),
                    }
                }
            }
            wayland_client::protocol::wl_keyboard::Event::Modifiers { mods_locked, .. } => {
                state.locks = lock_keys_from_xkb(mods_locked);
                if let Ok(mut locks) = state.shared_locks.lock() {
                    *locks = state.locks;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpRelativePointerV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _: &ZwpRelativePointerV1,
        event: RelativePointerEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let RelativePointerEvent::RelativeMotion {
            dx: raw_dx,
            dy: raw_dy,
            ..
        } = event
            && (state.grabbed || state.pointer_inside)
        {
            let (dx, dy) = state.relative_motion.update(raw_dx, raw_dy);
            debug!(
                position = ?state.position,
                raw_dx,
                raw_dy,
                dx,
                dy,
                grabbed = state.grabbed,
                inside = state.pointer_inside,
                "捕获层收到相对移动"
            );
            if dx != 0 || dy != 0 {
                state.emit(
                    CaptureEvent::Pointer {
                        position: state.position,
                        dx,
                        dy,
                    },
                    state.grabbed,
                );
            }
        }
    }
}

impl Dispatch<ZwpLockedPointerV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        locked: &ZwpLockedPointerV1,
        event: wayland_protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::Event::Locked => {
                if state.locked.as_ref().is_some_and(|active| active.id() == locked.id()) {
                    state.grab_pointer_locked = true;
                    debug!(keyboard_focused = state.grab_keyboard_focused, "指针锁定已确认");
                    state.request_keyboard_focus();
                    if state.keyboard_surface == Some(state.active) {
                        state.grab_keyboard_focused = true;
                    }
                    state.finish_grab_if_ready();
                }
            }
            wayland_protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::Event::Unlocked => {
                if state.locked.as_ref().is_some_and(|active| active.id() == locked.id()) {
                    if state.grabbed {
                        state.focus_lost("本机指针锁定丢失");
                    } else if state.grab_reply.is_some() {
                        state.fail_grab("niri 没有接受指针锁定");
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wayland_client::protocol::wl_compositor::WlCompositor, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_compositor::WlCompositor,
        _: wayland_client::protocol::wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlShm, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wayland_client::protocol::wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlBuffer, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: wayland_client::protocol::wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_shm_pool::WlShmPool,
        _: wayland_client::protocol::wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ZwlrLayerShellV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &ZwpKeyboardShortcutsInhibitManagerV1,
        _: wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpKeyboardShortcutsInhibitorV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        inhibitor: &ZwpKeyboardShortcutsInhibitorV1,
        event: ShortcutsInhibitorEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if state
            .shortcuts_inhibitor
            .as_ref()
            .is_none_or(|active| active.id() != inhibitor.id())
        {
            return;
        }
        match event {
            ShortcutsInhibitorEvent::Active => {
                state.grab_shortcuts_inhibited = true;
                debug!(output = state.active, "niri 快捷键抑制已生效");
                state.finish_grab_if_ready();
            }
            ShortcutsInhibitorEvent::Inactive => {
                state.grab_shortcuts_inhibited = false;
                warn!("niri 已恢复本机快捷键");
                if state.grabbed {
                    state.focus_lost("本机快捷键抑制被解除");
                } else if state.grab_reply.is_some() {
                    state.fail_grab("niri 没有接受快捷键抑制");
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpPointerConstraintsV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &ZwpPointerConstraintsV1,
        _: wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ZwpRelativePointerManagerV1, ()> for CaptureState {
    fn event(
        _: &mut Self,
        _: &ZwpRelativePointerManagerV1,
        _: wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<virtual_pointer::ZwlrVirtualPointerManagerV1, ()> for CaptureState {
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
impl Dispatch<virtual_pointer::ZwlrVirtualPointerV1, ()> for CaptureState {
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
