//! 服务端：拥有键鼠的一方。捕获本机键鼠，光标越过某一段边缘后把输入发给那一段的客户端。
//!
//! 可以同时连着几台客户端（ADR 0006）。主循环拥有本机键鼠捕获和布局，决定光标在哪台设备上、
//! 输入发给谁；每个会话一个任务，负责收发、心跳和往返延迟，主循环经队列把要发的消息交给它。
//!
//! 任何原因结束一个会话（对方断开、心跳超时、出错）时，光标在它上面就立即把控制权交还本机。
//! 光标在对方屏幕时，按 Ctrl + Option（Alt）+ Esc 可以强制回到本机。
//!
//! 每台设备可以单独设置（ADR 0009）：按键映射与文本编辑快捷键、滚动方向与速度、光标速度、
//! 切过去前在边上停多久、同不同步剪贴板、能不能拖文件给它。这些都在发给它之前、在本机这边处理。
//!
//! 所有连接都走 TLS 1.3。只有配对过的设备能建立会话；陌生设备会先发起配对，
//! 由两边的用户核对配对码。客户端握手后会用拿到的会话号再连一条数据连接，
//! 用来同步剪贴板和收发文件控制消息；每次传文件再连一条文件连接。

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use ferry_core::{KeyMapper, Neighbors, Place, Pressed, Side, Transition, WheelScale, bounds};
use ferry_net::discovery::Advertiser;
use ferry_net::{Framed, ServerStream, Tls};
use ferry_platform::{CaptureEvent, Captured, Clipboard, Displays, InputCapture};
use ferry_proto::{
    Bye, ByeReason, Capabilities, Channel, DropHint, Enter, Hello, Message, RejectReason,
    SecureAction, Welcome,
};
use ferry_store::Store;
use ferry_trust::{Fingerprint, InputSigner, Peer};
use ferry_types::{
    DeviceSettings, Display, InputEvent, KeyAction, KeyCode, Placement, Point, Rect, mark,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval, timeout};
use tracing::{debug, info, warn};

use crate::clipboard::{self, ClipLimit, ClipboardTask, Reporter, SharedClipboard, Start};
use crate::edge_drop::{Edge, EdgeFeed, gone};
use crate::error::{Error, Result};
use crate::events::{Event, Events};
use crate::files::Files;
use crate::heartbeat::{self, Heartbeat};
use crate::link::{
    DISPLAY_CHECK, HANDSHAKE_TIMEOUT, capabilities, frame_of, platform_of, primary_scale, send,
};
use crate::pairing::{self, ConfirmPairing, Outcome};

/// 主循环交给一个会话、还没发出去的消息最多积压多少条。积压满了说明对方长时间收不动。
const OUTBOX: usize = 1024;

pub struct ServerConfig {
    /// 会话中途也能立即生效的设置。
    pub live: watch::Receiver<Live>,
    /// 本机设备名，显示给对方。
    pub name: String,
    /// 是否同步剪贴板。
    pub clipboard: bool,
    /// 剪贴板单项的大小上限，会话中途也能改。
    pub clipboard_limit: ClipLimit,
    /// 本机的快捷键以 Command 键为主（macOS）。客户端与本机不同时，修饰键的“本机习惯”才互换 Command 与 Ctrl。
    pub command_key: bool,
    /// 数据目录：本机身份和已信任的设备。
    pub store: Store,
    /// 是否在局域网里用 mDNS 广播本机，让客户端不填地址也能找到。
    pub announce: bool,
    /// 对外报告的事件。
    pub events: Events,
    /// 文件传输。跨会话保留，断线重连后接着传。
    pub files: Files,
    /// 边缘投递：报告本机光标和朝向各台设备的边。
    pub edge: EdgeFeed,
}

/// 会话中途也能立即生效的设置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Live {
    /// 没有在 `layout` 里单独安排的设备放在本机哪一侧。
    pub side: Side,
    /// 单独安排过位置的设备，按先后排列：同一侧的几台按这个先后平分那条边。
    /// 光标所在的设备换到另一侧时，先回到本机。
    pub layout: Vec<Placement>,
    /// 单独设置过的设备。
    pub devices: Vec<DeviceSettings>,
    /// 没有单独设置过的设备用它（指纹不看）。命令行模式的 `--no-cmd-ctrl-swap` 用它把修饰键改为原样。
    pub fallback: DeviceSettings,
    /// 本机鼠标的 DPI，各台设备的光标速度以它为准换算。
    pub mouse_dpi: u32,
}

impl Live {
    /// 运行期间不会改变的设置。
    pub fn fixed(self) -> watch::Receiver<Self> {
        watch::channel(self).1
    }

    /// 一台设备的设置。
    fn device(&self, peer: &Fingerprint) -> DeviceSettings {
        let hex = peer.to_hex();
        self.devices
            .iter()
            .find(|d| d.fingerprint == hex)
            .cloned()
            .unwrap_or_else(|| DeviceSettings {
                fingerprint: hex,
                ..self.fallback.clone()
            })
    }

    /// 一台设备在布局里的位置：单独安排过的按安排的来，其余放在 `side` 一侧、排在后面。
    fn place_of(&self, peer: &Fingerprint) -> Place {
        let hex = peer.to_hex();
        match self.layout.iter().position(|p| p.fingerprint == hex) {
            Some(i) => Place {
                side: self.layout[i].side,
                rank: i as u32,
            },
            None => Place {
                side: self.side,
                rank: u32::MAX,
            },
        }
    }
}

/// 运行服务端：在 `listener` 上同时服务多台客户端，直到被丢弃。
///
/// 陌生设备发起配对时，用 `confirm` 请本机用户核对配对码。
/// 只有启动失败（例如捕获权限不足、数据目录不可用）时才返回错误。
pub async fn serve<C, D, K, P>(
    listener: TcpListener,
    config: ServerConfig,
    capture: C,
    displays: D,
    clipboard: K,
    confirm: P,
) -> Result<Infallible>
where
    C: InputCapture,
    D: Displays,
    K: Clipboard,
    P: ConfirmPairing,
{
    let mut server = Server::start(config, capture, displays, clipboard, confirm)?;
    let addr = listener.local_addr()?;
    // 广播失败（例如网络不支持组播）不影响按地址连接。
    let advertiser = if server.config.announce {
        match Advertiser::start(&server.config.name, server.shared.fingerprint, addr.port()) {
            Ok(advertiser) => Some(advertiser),
            Err(e) => {
                warn!("{e}；客户端需要手动指定本机地址");
                None
            }
        }
    } else {
        None
    };
    info!(
        %addr,
        fingerprint = %server.shared.fingerprint,
        announced = advertiser.is_some(),
        "等待客户端连接",
    );
    server.config.events.emit(Event::Listening {
        port: addr.port(),
        announced: advertiser.is_some(),
    });
    server.run(listener).await
}

/// 会话任务和连接握手交回主循环的事。
enum Arrival {
    /// 一台已配对的设备完成了握手和显示器交换，开始会话。
    Session(Box<NewSession>),
    /// 会话的数据连接到了。
    Data {
        id: u64,
        framed: Box<Framed<ServerStream>>,
    },
    /// 对方在会话里发来的、要主循环处理的消息（显示器变化、拖动提示）。
    Message { id: u64, message: Message },
    /// 会话结束。`reason` 为 `None` 表示对方正常断开。
    Closed { id: u64, reason: Option<String> },
}

struct NewSession {
    id: u64,
    peer: Fingerprint,
    addr: SocketAddr,
    hello: Hello,
    welcome: Welcome,
    remote: Vec<Display>,
    framed: Framed<ServerStream>,
    signed: Option<SignedOutput>,
}

/// 连接握手（在各自的任务里进行）需要的东西。
struct Shared<P> {
    capabilities: Capabilities,
    tls: Tls,
    signer: Arc<InputSigner>,
    fingerprint: Fingerprint,
    name: String,
    store: Store,
    events: Events,
    files: Files,
    confirm: P,
    next_id: AtomicU64,
    /// 正在进行的会话：会话号 → 对方与协商结果。数据连接和文件连接凭它认领。
    sessions: Mutex<HashMap<u64, Keys>>,
    /// 本机最新的显示器，握手时发给对方。
    local: Mutex<Vec<Display>>,
    arrivals: mpsc::Sender<Arrival>,
}

#[derive(Clone, Copy)]
struct Keys {
    peer: Fingerprint,
    /// 双方是否都开启了剪贴板同步（本机对这台设备也没有关掉）。
    clipboard: bool,
    /// 双方是否都支持传文件。
    files: bool,
}

impl<P> Shared<P> {
    fn sessions(&self) -> MutexGuard<'_, HashMap<u64, Keys>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn local(&self) -> MutexGuard<'_, Vec<Display>> {
        self.local.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// 丢弃即停止的后台任务。
struct Task(JoinHandle<()>);

impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 一个正在进行的会话。
struct Session {
    peer: Fingerprint,
    name: String,
    /// 交给会话任务发出的消息。
    outbox: mpsc::Sender<Message>,
    /// 对这台设备而言本机是不是活动端：光标不在它上面。剪贴板同步和心跳据此判断。
    active: watch::Sender<bool>,
    remote: Vec<Display>,
    keys: Keys,
    /// 对方懂边缘投递的消息（`PeerSide`、`DropHint`）。
    edge_drop: bool,
    /// 对方的快捷键以 Command 键为主（macOS）。
    remote_command: bool,
    /// 本机对这台设备的单独设置，以及据此做的按键和滚轮换算。
    settings: DeviceSettings,
    mapper: KeyMapper,
    wheel: WheelScale,
    /// 最近一次告诉它的“服务端在你的哪一侧”。
    side: Side,
    _io: Task,
    /// 剪贴板同步和文件控制消息，数据连接到了才有。
    sync: Option<ClipboardTask>,
}

struct Server<C, D, K, P> {
    config: ServerConfig,
    shared: Arc<Shared<P>>,
    arrivals: mpsc::Receiver<Arrival>,
    capture: C,
    displays: D,
    clipboard: SharedClipboard<K>,
    events: UnboundedReceiver<Captured>,
    local: Vec<Display>,
    neighbors: Neighbors,
    sessions: HashMap<u64, Session>,
    /// 按着的鼠标键和（光标在对方屏幕时）按着的键。
    /// 按着鼠标键时不切换屏幕，避免拖拽到一半换了屏；按着的键用来识别快捷键。
    pressed: Pressed,
    /// 发消息时发现收不动的会话，本轮处理完后断开。
    broken: Vec<u64>,
    /// 最近一次连着的能收文件的设备；一台都没连着时据此摆出“未连接”的边。
    known: Vec<(Fingerprint, String)>,
    /// 拖动提示最近发给了哪台设备。
    hint_target: Option<Fingerprint>,
}

impl<C, D, K, P> Server<C, D, K, P>
where
    C: InputCapture,
    D: Displays,
    K: Clipboard,
    P: ConfirmPairing,
{
    fn start(
        config: ServerConfig,
        mut capture: C,
        displays: D,
        clipboard: K,
        confirm: P,
    ) -> Result<Self> {
        let identity = config.store.identity(&config.name)?;
        let tls = Tls::new(&identity)?;
        let signer = Arc::new(InputSigner::new(&identity)?);
        let (tx, events) = mpsc::unbounded_channel();
        capture.start(Box::new(move |event| {
            let _ = tx.send(event);
        }))?;
        let local = displays.displays()?;
        let rect = bounds(&local).ok_or(Error::NoDisplays("本机"))?;
        let (arrivals_tx, arrivals) = mpsc::channel(64);
        let shared = Arc::new(Shared {
            capabilities: capabilities(config.clipboard, config.command_key),
            tls,
            signer,
            fingerprint: identity.fingerprint(),
            name: config.name.clone(),
            store: config.store.clone(),
            events: config.events.clone(),
            files: config.files.clone(),
            confirm,
            next_id: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
            local: Mutex::new(local.clone()),
            arrivals: arrivals_tx,
        });
        Ok(Self {
            config,
            shared,
            arrivals,
            capture,
            displays,
            clipboard: Arc::new(Mutex::new(clipboard)),
            events,
            local,
            neighbors: Neighbors::new(rect),
            sessions: HashMap::new(),
            pressed: Pressed::new(),
            broken: Vec::new(),
            known: Vec::new(),
            hint_target: None,
        })
    }

    async fn run(&mut self, listener: TcpListener) -> Result<Infallible> {
        let mut screens = interval(DISPLAY_CHECK);
        screens.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut hints = self.config.edge.hints();
        // 设置的发送方已经不在（命令行模式）时，不再等它的变化。
        let mut live_open = true;
        loop {
            tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok((stream, addr)) => {
                        tokio::spawn(connection(self.shared.clone(), stream, addr));
                    }
                    Err(e) => warn!("接受连接失败：{e}"),
                },
                Some(arrival) = self.arrivals.recv() => self.on_arrival(arrival),
                Some(captured) = self.events.recv() => self.on_capture(captured),
                changed = self.config.live.changed(), if live_open => {
                    if changed.is_err() {
                        live_open = false;
                        continue;
                    }
                    self.relayout();
                }
                changed = hints.changed() => {
                    if changed.is_ok() {
                        let hint = hints.borrow_and_update().clone();
                        self.route_hint(hint);
                    }
                }
                _ = screens.tick() => self.refresh_local(),
            }
            for id in std::mem::take(&mut self.broken) {
                self.end(id, Some("对方长时间收不动，已断开".into()));
            }
        }
    }

    // ── 会话的开始与结束 ─────────────────────────

    fn on_arrival(&mut self, arrival: Arrival) {
        match arrival {
            Arrival::Session(new) => self.begin(*new),
            Arrival::Data { id, framed } => self.attach_data(id, *framed),
            Arrival::Message { id, message } => self.on_message(id, message),
            Arrival::Closed { id, reason } => self.end(id, reason),
        }
    }

    fn begin(&mut self, new: NewSession) {
        let NewSession {
            id,
            peer,
            addr,
            hello,
            welcome,
            remote,
            framed,
            signed,
        } = new;
        // 同一台设备重连：新会话顶替旧会话。
        let stale: Vec<u64> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.peer == peer)
            .map(|(id, _)| *id)
            .collect();
        for old in stale {
            info!(peer = %mark::device(&hello.name), "这台设备重新连上了，结束它之前的会话");
            self.end(old, None);
        }
        let Some(remote_rect) = bounds(&remote) else {
            warn!(peer = %mark::device(&hello.name), "对方没有报告显示器，已断开");
            return;
        };
        let (place, settings, mouse_dpi) = {
            let live = self.config.live.borrow();
            (live.place_of(&peer), live.device(&peer), live.mouse_dpi)
        };
        let scale = relative_scale(&self.local, &remote);
        self.neighbors.insert(id, place, remote_rect, scale);
        self.apply_pointer(id, &settings, mouse_dpi);
        let caps = welcome.capabilities;
        let keys = Keys {
            peer,
            clipboard: caps.contains(Capabilities::CLIPBOARD) && settings.clipboard,
            files: caps.contains(Capabilities::FILES),
        };
        self.shared.sessions().insert(id, keys);

        let (outbox, out_rx) = mpsc::channel(OUTBOX);
        let active = watch::Sender::new(true);
        let io = tokio::spawn(run_session(
            framed,
            id,
            (peer, signed),
            out_rx,
            active.subscribe(),
            self.shared.arrivals.clone(),
            self.config.events.clone(),
        ));
        let edge_drop = caps.contains(Capabilities::EDGE_DROP);
        // 看对方自己声明的那一位，不看交集：交集里只有两边都是 Mac 时才有它。
        let remote_command = hello.capabilities.contains(Capabilities::COMMAND_KEY);
        let session = Session {
            peer,
            name: hello.name.clone(),
            outbox,
            active,
            remote,
            keys,
            edge_drop,
            remote_command,
            mapper: KeyMapper::new(&settings, self.config.command_key, remote_command),
            wheel: WheelScale::new(&settings),
            settings,
            side: place.side,
            _io: Task(io),
            sync: None,
        };
        self.sessions.insert(id, session);
        if edge_drop {
            let side = place.side.opposite();
            self.send_to(id, Message::PeerSide { side });
        }
        info!(
            peer = %mark::device(&hello.name),
            fingerprint = %peer,
            remote = ?remote_rect,
            scale,
            "握手完成：把光标推出本机 {:?} 侧边缘即可切换过去",
            place.side,
        );
        self.config.events.emit(Event::Connected {
            name: hello.name,
            fingerprint: peer,
            addr,
            platform: Some(platform_of(remote_command)),
        });
        self.publish_edges();
    }

    fn attach_data(&mut self, id: u64, framed: Framed<ServerStream>) {
        let Some(session) = self.sessions.get_mut(&id) else {
            return;
        };
        info!(peer = %mark::device(&session.name), "数据连接已建立");
        let keys = session.keys;
        let report = Reporter {
            peer: keys.peer,
            events: self.config.events.clone(),
            limit: self.config.clipboard_limit.clone(),
        };
        let files = keys
            .files
            .then(|| self.config.files.attach(keys.peer, None));
        let task = clipboard::spawn(
            framed,
            self.clipboard.clone(),
            session.active.subscribe(),
            Start::Announce,
            report,
            keys.clipboard,
            files,
        );
        session.sync = Some(task);
    }

    fn on_message(&mut self, id: u64, message: Message) {
        let Some(session) = self.sessions.get_mut(&id) else {
            return;
        };
        match message {
            Message::Displays(displays) => {
                session.remote = displays;
                if let Some(rect) = bounds(&session.remote) {
                    let scale = relative_scale(&self.local, &session.remote);
                    self.neighbors.set_remote(id, rect, scale);
                    info!(peer = %mark::device(&session.name), remote = ?rect, "对方的显示器布局已更新");
                }
            }
            Message::DropHint(hint) => self.config.edge.remote(session.peer, Some(hint)),
            Message::OpenSettings => {
                info!(peer = %mark::device(&session.name), "对方请本机打开设置界面");
                self.config
                    .events
                    .emit(Event::OpenSettings { peer: session.peer });
            }
            Message::AppInfo { version } => {
                info!(peer = %mark::device(&session.name), %version, "对方的版本");
                self.config.events.emit(Event::PeerVersion {
                    peer: session.peer,
                    version,
                });
            }
            other => debug!(kind = format_args!("{:#06x}", other.type_id()), "忽略消息"),
        }
    }

    /// 结束一个会话。光标在它上面时立即交还本机。
    fn end(&mut self, id: u64, reason: Option<String>) {
        let Some(session) = self.sessions.remove(&id) else {
            return;
        };
        self.shared.sessions().remove(&id);
        if let Some(at) = self.neighbors.remove(id) {
            self.pressed.clear_keys();
            match self.capture.release(at) {
                Ok(()) => info!("已把控制权交还本机"),
                Err(e) => warn!("交还本机控制失败：{e}"),
            }
            self.config.edge.pointer(|p| p.here = true);
        }
        match &reason {
            None => info!(peer = %mark::device(&session.name), "客户端已断开"),
            Some(reason) => warn!(peer = %mark::device(&session.name), "连接中断：{reason}"),
        }
        if self.hint_target == Some(session.peer) {
            self.hint_target = None;
        }
        self.config.edge.remote(session.peer, None);
        self.config.events.emit(Event::Disconnected {
            peer: session.peer,
            reason,
        });
        drop(session);
        self.publish_edges();
    }

    // ── 本机输入 ──────────────────────────────

    fn on_capture(&mut self, captured: Captured) {
        // 切换那一刻还在路上的事件，发生时光标在另一边：本机已经处理过它，或者它已被吞掉。
        // 按现在的状态解释它，会拿过时的位置判断边缘、把光标立刻推回去，所以不再使用；
        // 鼠标键照样记录，免得以为某个键一直按着。
        let remote = self.neighbors.active();
        let current = captured.grabbed == remote.is_some();

        match captured.event {
            CaptureEvent::Pointer { position, dx, dy } => {
                if !current {
                    return;
                }
                let free = !self.pressed.any_button();
                if remote.is_none() {
                    self.config.edge.pointer(|p| {
                        p.x = position.x as f32;
                        p.y = position.y as f32;
                        p.here = true;
                    });
                    if free
                        && let Some((id, t)) =
                            self.neighbors
                                .local_motion(position, dx, dy, Instant::now())
                    {
                        self.perform(id, t, false);
                    }
                } else if let Some((id, t)) =
                    self.neighbors.remote_motion(dx, dy, free, Instant::now())
                {
                    self.perform(id, t, false);
                }
            }
            CaptureEvent::Button { button, pressed } => {
                self.neighbors.reset_wait();
                let event = InputEvent::Button {
                    button,
                    pressed,
                    position: Point::default(),
                };
                self.pressed.track(&event);
                let held = self.pressed.any_button();
                self.config.edge.pointer(|p| p.held = held);
                if current
                    && let (Some(id), Some(position)) = (remote, self.neighbors.remote_position())
                {
                    let event = InputEvent::Button {
                        button,
                        pressed,
                        position,
                    };
                    self.send_to(id, Message::Input(event));
                }
            }
            CaptureEvent::Wheel { dx, dy } => {
                if current
                    && let (Some(id), Some(position)) = (remote, self.neighbors.remote_position())
                    && let Some(session) = self.sessions.get_mut(&id)
                {
                    let (dx, dy) = session.wheel.apply(dx, dy);
                    if dx != 0 || dy != 0 {
                        let event = InputEvent::Wheel { dx, dy, position };
                        self.send_to(id, Message::Input(event));
                    }
                }
            }
            CaptureEvent::Key { code, action } => {
                // 光标在本机时，按键由本机自己处理。
                let Some(id) = remote else {
                    return;
                };
                self.pressed.track(&InputEvent::Key { code, action });
                if action == KeyAction::Down
                    && code == KeyCode::ESCAPE
                    && escape_hotkey(&self.pressed)
                {
                    info!("快捷键 Ctrl + Option（Alt）+ Esc：回到本机");
                    if let Some((id, t)) = self.neighbors.leave() {
                        self.perform(id, t, true);
                    }
                } else if let Some(session) = self.sessions.get_mut(&id) {
                    for (code, action) in session.mapper.key(code, action) {
                        self.send_to(id, Message::Input(InputEvent::Key { code, action }));
                    }
                }
            }
        }
    }

    /// 执行一次切换或移动。
    fn perform(&mut self, id: u64, transition: Transition, hotkey: bool) {
        let Some(peer) = self.sessions.get(&id).map(|s| s.peer) else {
            return;
        };
        match transition {
            Transition::Enter { seq, at } => {
                // 先抓取再通知对方：抓取失败就不过去。
                if let Err(e) = self.capture.grab() {
                    warn!("抓取本机键鼠失败，留在本机：{e}");
                    self.neighbors.abort();
                    return;
                }
                self.config.edge.pointer(|p| p.here = false);
                let enter = Enter {
                    seq,
                    position: at,
                    locks: self.capture.lock_keys(),
                };
                self.send_to(id, Message::Enter(enter));
                if let Some(session) = self.sessions.get(&id) {
                    session.active.send_replace(false);
                    info!(peer = %mark::device(&session.name), ?at, "光标进入对方屏幕");
                }
                self.config.events.emit(Event::Cursor {
                    here: false,
                    hotkey: false,
                    peer,
                });
            }
            Transition::Move(position) => {
                self.send_to(id, Message::Input(InputEvent::PointerMotion { position }));
            }
            Transition::Leave { seq, local_at } => {
                // 先归还本机控制再通知对方：发送失败也不会把本机键鼠锁住。
                if let Err(e) = self.capture.release(local_at) {
                    warn!("交还本机控制失败：{e}");
                }
                self.config.edge.pointer(|p| p.here = true);
                // 对方收到 Leave 会松开所有键，这些键不再需要跟踪。
                self.pressed.clear_keys();
                if let Some(session) = self.sessions.get_mut(&id) {
                    session.mapper.reset();
                    session.active.send_replace(true);
                }
                self.send_to(id, Message::Leave { seq });
                info!("光标回到本机");
                self.config.events.emit(Event::Cursor {
                    here: true,
                    hotkey,
                    peer,
                });
            }
        }
    }

    /// 交给会话任务发出。队列满了说明对方长时间收不动，本轮结束后断开它。
    fn send_to(&mut self, id: u64, message: Message) {
        let Some(session) = self.sessions.get(&id) else {
            return;
        };
        if session.outbox.try_send(message).is_err() && !self.broken.contains(&id) {
            self.broken.push(id);
        }
    }

    // ── 布局、显示器与边缘投递 ─────────────────────

    /// 用户改了布局或某台设备的设置：每台设备按新的位置摆、按新的设置换算。
    /// 光标所在的那台换到另一侧时先回到本机。
    fn relayout(&mut self) {
        let live = self.config.live.borrow_and_update().clone();
        let ids: Vec<u64> = self.sessions.keys().copied().collect();
        for id in ids {
            let Some(session) = self.sessions.get_mut(&id) else {
                continue;
            };
            let settings = live.device(&session.peer);
            if settings != session.settings {
                info!(peer = %mark::device(&session.name), "这台设备的设置变了");
                // 改设置要点本机的设置界面，那时光标在本机、对方没有按着的键，直接换成新的。
                // 万一还按着，光标离开时对方也会松开所有键。
                session.mapper =
                    KeyMapper::new(&settings, self.config.command_key, session.remote_command);
                session.wheel = WheelScale::new(&settings);
                session.settings = settings.clone();
            }
            self.apply_pointer(id, &settings, live.mouse_dpi);
            let Some(session) = self.sessions.get(&id) else {
                continue;
            };
            let place = live.place_of(&session.peer);
            if let Some(t) = self.neighbors.set_place(id, place) {
                self.perform(id, t, false);
            }
            let Some(session) = self.sessions.get_mut(&id) else {
                continue;
            };
            if session.side != place.side {
                info!(peer = %mark::device(&session.name), side = ?place.side, "这台设备换了位置");
                session.side = place.side;
                if session.edge_drop {
                    let side = place.side.opposite();
                    self.send_to(id, Message::PeerSide { side });
                }
            }
        }
        self.publish_edges();
    }

    /// 本机的显示器变了（改分辨率、插拔显示器）：重新划分各段，告诉每台设备。
    fn refresh_local(&mut self) {
        // 查询偶尔失败（例如显示器正在切换）时等下一次。
        let Ok(now) = self.displays.displays() else {
            return;
        };
        if now == self.local {
            return;
        }
        let Some(rect) = bounds(&now) else {
            return;
        };
        self.local = now;
        *self.shared.local() = self.local.clone();
        self.neighbors.set_local(rect);
        info!(local = ?rect, "本机的显示器布局已更新");
        let ids: Vec<u64> = self.sessions.keys().copied().collect();
        for id in ids {
            if let Some(session) = self.sessions.get(&id)
                && let Some(remote) = bounds(&session.remote)
            {
                let scale = relative_scale(&self.local, &session.remote);
                self.neighbors.set_remote(id, remote, scale);
            }
            self.send_to(id, Message::Displays(self.local.clone()));
        }
        self.publish_edges();
    }

    /// 按一台设备的设置调整光标速度，以及往返切换前在边上停多久。
    fn apply_pointer(&mut self, id: u64, settings: &DeviceSettings, mouse_dpi: u32) {
        let base = mouse_dpi.max(1);
        let dpi = match settings.pointer_dpi {
            0 => base,
            dpi => dpi,
        };
        self.neighbors.set_speed(id, dpi, base);
        self.neighbors
            .set_delay(id, Duration::from_millis(u64::from(settings.edge_delay_ms)));
    }

    /// 告诉边缘投递朝向各台能收文件的设备的那一段边。一台都没连着时，
    /// 照最后连着的那些设备和现在的布局摆出“未连接”的边。
    fn publish_edges(&mut self) {
        let scale = f32::from(primary_scale(&self.local)) / 100.0;
        let mut online: Vec<(u64, &Session)> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.keys.files && s.settings.files)
            .map(|(id, s)| (*id, s))
            .collect();
        online.sort_by_key(|(id, _)| *id);
        let edges: Vec<Edge> = if online.is_empty() {
            let Some(rect) = bounds(&self.local) else {
                return;
            };
            let live = self.config.live.borrow();
            let mut layout = Neighbors::new(rect);
            for (i, (peer, _)) in self.known.iter().enumerate() {
                layout.insert(i as u64, live.place_of(peer), Rect::new(0, 0, 1, 1), 100);
            }
            self.known
                .iter()
                .enumerate()
                .filter_map(|(i, (peer, name))| {
                    let (side, segment) = layout.segment(i as u64)?;
                    Some(Edge {
                        id: *peer,
                        side,
                        screen: frame_of(segment),
                        scale,
                        peer: name.clone(),
                        online: false,
                    })
                })
                .collect()
        } else {
            let edges = online
                .iter()
                .filter_map(|(id, s)| {
                    let (side, segment) = self.neighbors.segment(*id)?;
                    Some(Edge {
                        id: s.peer,
                        side,
                        screen: frame_of(segment),
                        scale,
                        peer: s.display_name(),
                        online: true,
                    })
                })
                .collect();
            self.known = online
                .iter()
                .map(|(_, s)| (s.peer, s.display_name()))
                .collect();
            edges
        };
        self.config.edge.edges(edges);
    }

    /// 把本机拖动中的提示发给拖向的那台设备；换了设备时先让原来那台收起。
    fn route_hint(&mut self, hint: Option<(Fingerprint, DropHint)>) {
        let target = hint.as_ref().map(|(peer, _)| *peer);
        if let Some(old) = self.hint_target
            && Some(old) != target
        {
            self.hint_to(old, gone());
        }
        self.hint_target = target;
        if let Some((peer, hint)) = hint {
            self.hint_to(peer, hint);
        }
    }

    fn hint_to(&mut self, peer: Fingerprint, hint: DropHint) {
        let id = self
            .sessions
            .iter()
            .find(|(_, s)| s.peer == peer && s.edge_drop)
            .map(|(id, _)| *id);
        if let Some(id) = id {
            self.send_to(id, Message::DropHint(hint));
        }
    }
}

/// 处理一条新连接：TLS 握手，然后按对方声明的连接类型配对、开始会话，或者认领到已有的会话上。
async fn connection<P: ConfirmPairing>(
    shared: Arc<Shared<P>>,
    stream: TcpStream,
    addr: SocketAddr,
) {
    if let Err(e) = handshake(&shared, stream, addr).await {
        warn!(%addr, "连接中断：{e}");
    }
}

async fn handshake<P: ConfirmPairing>(
    shared: &Shared<P>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<()> {
    let (mut framed, peer) = timeout(HANDSHAKE_TIMEOUT, shared.tls.accept(stream))
        .await
        .map_err(|_| Error::Timeout("TLS 握手"))??;
    // 每次都从磁盘读：用户手动删掉某台设备后立即生效。
    let peers = shared.store.peers()?;
    let accepted = ferry_net::accept(&mut framed, shared.capabilities, &shared.name, |hello| {
        match hello.channel {
            Channel::Input if peers.contains(&peer) => {
                Ok(shared.next_id.fetch_add(1, Ordering::Relaxed))
            }
            Channel::Input => Err(RejectReason::NOT_PAIRED),
            Channel::Pair => Ok(0),
            // 本机是服务端，不等邀请。
            Channel::Invite => Err(RejectReason::UNKNOWN_SESSION),
            // 数据连接和文件连接必须来自会话的同一台设备（证书指纹相同），并带着那个会话号。
            Channel::Data | Channel::File => {
                let keys = shared.sessions().get(&hello.session).copied();
                let claimed = keys.is_some_and(|k| {
                    k.peer == peer
                        && match hello.channel {
                            Channel::File => k.files,
                            _ => k.clipboard || k.files,
                        }
                });
                if claimed {
                    Ok(hello.session)
                } else {
                    Err(RejectReason::UNKNOWN_SESSION)
                }
            }
        }
    });
    let (hello, welcome) = match timeout(HANDSHAKE_TIMEOUT, accepted).await {
        Err(_) => return Err(Error::Timeout("握手")),
        Ok(Err(ferry_net::Error::Rejected(r))) if r.reason == RejectReason::NOT_PAIRED => {
            info!(fingerprint = %peer, "一台未配对的设备想要连接，已请它先配对");
            return Ok(());
        }
        Ok(result) => result?,
    };
    match hello.channel {
        Channel::Invite => Ok(()),
        Channel::Pair => pair(shared, &mut framed, peer, addr, &hello.name).await,
        Channel::File => {
            shared.files.serve(peer, framed);
            Ok(())
        }
        Channel::Data => {
            let id = welcome.session;
            let framed = Box::new(framed);
            let _ = shared.arrivals.send(Arrival::Data { id, framed }).await;
            Ok(())
        }
        Channel::Input => {
            info!(peer = %mark::device(&hello.name), fingerprint = %peer, "客户端已连接");
            // 记下它的 IP：以后本机开始共享时邀请它。记不下来只是少一个兜底。
            if let Err(e) = remember_addr(&shared.store, peer, SocketAddr::new(addr.ip(), 0)) {
                warn!("记下客户端的地址失败：{e}");
            }
            let local = shared.local().clone();
            send(&mut framed, &Message::Displays(local)).await?;
            let remote = timeout(HANDSHAKE_TIMEOUT, expect_displays(&mut framed))
                .await
                .map_err(|_| Error::Timeout("等待对方的显示器信息"))??;
            let signed = if welcome.capabilities.contains(Capabilities::SECURE_INPUT) {
                match timeout(HANDSHAKE_TIMEOUT, framed.recv())
                    .await
                    .map_err(|_| Error::Timeout("等待安全输入挑战"))??
                {
                    Some(Message::SecureChallenge { nonce }) => Some(SignedOutput {
                        signer: shared.signer.clone(),
                        nonce,
                        next: 1,
                    }),
                    _ => return Err(Error::Protocol("安全输入协商缺少代理挑战")),
                }
            } else {
                None
            };
            if welcome.capabilities.contains(Capabilities::APP_INFO) {
                send(&mut framed, &crate::link::app_info()).await?;
            }
            let new = NewSession {
                id: welcome.session,
                peer,
                addr,
                hello,
                welcome,
                remote,
                framed,
                signed,
            };
            let _ = shared.arrivals.send(Arrival::Session(Box::new(new))).await;
            Ok(())
        }
    }
}

async fn pair<P: ConfirmPairing>(
    shared: &Shared<P>,
    framed: &mut Framed<ServerStream>,
    peer: Fingerprint,
    addr: SocketAddr,
    name: &str,
) -> Result<()> {
    info!(peer = %mark::device(name), fingerprint = %peer, "收到配对请求");
    let outcome =
        pairing::as_acceptor(framed, shared.fingerprint, peer, name, &shared.confirm).await?;
    match outcome {
        Outcome::Paired => {
            let mut peers = shared.store.peers()?;
            // 只知道它的 IP：它是来连本机的，端口是临时的。
            peers.add(Peer {
                name: name.to_owned(),
                fingerprint: peer,
                addr: Some(SocketAddr::new(addr.ip(), 0)),
            });
            shared.store.save_peers(&peers)?;
            info!(peer = %mark::device(name), "配对成功，已记住这台设备");
            shared.events.emit(Event::Paired {
                name: name.to_owned(),
                fingerprint: peer,
            });
        }
        Outcome::RejectedHere | Outcome::RejectedByPeer => {
            let by_peer = outcome == Outcome::RejectedByPeer;
            if by_peer {
                info!(peer = %mark::device(name), "对方拒绝了配对");
            } else {
                info!(peer = %mark::device(name), "已拒绝配对");
            }
            shared.events.emit(Event::PairingRejected {
                name: name.to_owned(),
                by_peer,
            });
        }
    }
    Ok(())
}

struct SignedOutput {
    signer: Arc<InputSigner>,
    nonce: [u8; 32],
    next: u64,
}
impl SignedOutput {
    fn packet(&mut self, action: SecureAction) -> Result<Message> {
        let body = action
            .encode()
            .map_err(|_| Error::Protocol("签署输入动作不能编码"))?;
        let proof = self.signer.sign(self.nonce, self.next, body)?;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(Error::Protocol("远端输入序号耗尽"))?;
        Ok(Message::SecureInput(proof))
    }
}

/// 一个会话的收发：发出主循环交来的消息，回应心跳、测往返延迟，把要处理的消息交回主循环。
/// 结束时（对方断开、心跳超时、出错、主循环不要它了）告诉主循环。
async fn run_session(
    mut framed: Framed<ServerStream>,
    id: u64,
    security: (Fingerprint, Option<SignedOutput>),
    mut outbox: mpsc::Receiver<Message>,
    active: watch::Receiver<bool>,
    arrivals: mpsc::Sender<Arrival>,
    events: Events,
) {
    let (peer, mut signed) = security;
    let mut signed_at = Instant::now();
    let mut heartbeat = Heartbeat::new();
    let mut tick = interval(heartbeat::TICK);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut input_reported = Instant::now();
    let mut sent_inputs = 0_u64;
    let result: Result<()> = async {
        loop {
            tokio::select! {
                message = outbox.recv() => match message {
                    Some(message) => {
                        if matches!(message, Message::Input(_)) { sent_inputs += 1; }
                        let message = match &mut signed {
                            Some(output) => match SecureAction::from_message(&message) {
                                Some(action) => output.packet(action)?,
                                None => message,
                            },
                            None => message,
                        };
                        send(&mut framed, &message).await?;
                    }
                    None => return Ok(()),
                },
                message = framed.recv() => match message? {
                    Some(Message::SecureChallenge { nonce }) => {
                        let output = signed.as_mut().ok_or(Error::Protocol("未协商安全输入"))?;
                        output.nonce = nonce;
                    }
                    Some(Message::Pong { nonce }) => {
                        if let Some(rtt) = heartbeat.pong(nonce) {
                            events.emit(Event::Rtt { peer, rtt });
                        }
                    }
                    Some(Message::Ping { nonce }) => {
                        send(&mut framed, &Message::Pong { nonce }).await?;
                    }
                    Some(Message::Bye(_)) | None => return Ok(()),
                    Some(
                        message @ (Message::Displays(_)
                        | Message::DropHint(_)
                        | Message::OpenSettings
                        | Message::AppInfo { .. }),
                    ) => {
                        let _ = arrivals.send(Arrival::Message { id, message }).await;
                    }
                    Some(other) => debug!(kind = format_args!("{:#06x}", other.type_id()), "忽略消息"),
                },
                _ = tick.tick() => {
                    if signed_at.elapsed() >= Duration::from_millis(250) {
                        if let Some(output) = &mut signed { send(&mut framed, &output.packet(SecureAction::Alive)?).await?; }
                        signed_at = Instant::now();
                    }
                    if input_reported.elapsed() >= Duration::from_secs(5) {
                        debug!(session = id, sent_inputs, remote = !*active.borrow(), "输入转发统计（过去 5 秒）");
                        sent_inputs = 0;
                        input_reported = Instant::now();
                    }
                    if heartbeat.timed_out() {
                        let bye = Bye { reason: ByeReason::TIMEOUT };
                        let _ = send(&mut framed, &Message::Bye(bye)).await;
                        return Err(Error::HeartbeatTimeout);
                    }
                    let remote = !*active.borrow();
                    if let Some(nonce) = heartbeat.poll_ping(remote) {
                        send(&mut framed, &Message::Ping { nonce }).await?;
                    }
                    if let Some(report) = heartbeat.poll_report() {
                        info!("{report}");
                    }
                }
            }
        }
    }
    .await;
    let reason = result.err().map(|e| e.to_string());
    let _ = arrivals.send(Arrival::Closed { id, reason }).await;
}

impl Session {
    /// 显示给用户的名字：本机给它起过名字就用起的名字。
    fn display_name(&self) -> String {
        match self.settings.alias.trim() {
            "" => self.name.clone(),
            alias => alias.to_owned(),
        }
    }
}

/// 记下一台已配对设备这次连上的地址（有变化才写）。
pub(crate) fn remember_addr(store: &Store, peer: Fingerprint, addr: SocketAddr) -> Result<()> {
    let mut peers = store.peers()?;
    if peers.set_addr(&peer, addr) {
        store.save_peers(&peers)?;
    }
    Ok(())
}

/// Ctrl + Option（Alt）是否都按着。左右两侧的键都算。
fn escape_hotkey(pressed: &Pressed) -> bool {
    let held = |a, b| pressed.is_held(a) || pressed.is_held(b);
    held(KeyCode::LEFT_CTRL, KeyCode::RIGHT_CTRL) && held(KeyCode::LEFT_ALT, KeyCode::RIGHT_ALT)
}

async fn expect_displays(framed: &mut Framed<ServerStream>) -> Result<Vec<Display>> {
    match framed.recv().await? {
        Some(Message::Displays(displays)) => Ok(displays),
        Some(_) => Err(Error::Protocol("握手后应先发送显示器信息")),
        None => Err(ferry_net::Error::Closed.into()),
    }
}

/// 本机移动一个坐标单位，对方移动多少个坐标单位（百分数）。
fn relative_scale(local: &[Display], remote: &[Display]) -> u16 {
    let ratio = u32::from(primary_scale(remote)) * 100 / u32::from(primary_scale(local));
    u16::try_from(ratio).unwrap_or(u16::MAX)
}
