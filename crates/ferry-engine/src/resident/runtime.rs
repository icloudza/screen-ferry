//! 常驻运行的状态：引擎的启停、引擎事件与界面命令的处理、给界面的状态快照。

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::fs;
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ferry_core::Redactor;

use ferry_ipc::{
    ClipboardImage, ClipboardSync, DialError as IpcDialError, FoundDevice, Link, LinkStatus, Local,
    LogIcon, LogKind, Notice, PairingPrompt, PeerInfo, PeerRef, Phrase, Problem, ProblemKind,
    Request, Settings, SettingsPatch, State,
};
use ferry_store::Store;
use ferry_trust::{Fingerprint, PeerList};
use ferry_types::{
    ClipboardFormat, Config, DeviceSettings, Lang, Role, Side, mark, uses_command_key,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use super::confirm::{PairAsk, UiConfirm};
use super::diagnostics;
use super::record::{Activity, Log, Rtt, now_ms};
use super::ui::UiEvent;
use super::update::{Outcome, UpdateEvent, Updates};
use super::{Command, Host, Summary};
use crate::client::{ClientCommand, ClientConfig, ServerAddr, connect};
use crate::clipboard::ClipLimit;
use crate::edge_drop::EdgeDrop;
use crate::error::{Error, Result};
use crate::events::{DialError, Event, Events, FoundServer, TransferState, TransferUpdate};
use crate::files::Files;
use crate::invite::{INVITE_PORT, invite};
use crate::link::DEFAULT_PORT;
use crate::server::{Live, ServerConfig, serve};

/// 缺少权限时多久再试一次：用户在系统设置里勾上之后很快就能用。
const RETRY_PERMISSION: Duration = Duration::from_secs(2);
/// 配对被拒绝后多久再试。
const RETRY_PAIRING: Duration = Duration::from_secs(3);
/// 其他问题多久再试。
const RETRY_OTHER: Duration = Duration::from_secs(5);
/// 多久重新读一次已配对设备（用户可能用命令行删了设备）。
const PEERS_REFRESH: Duration = Duration::from_secs(5);
/// 另一台设备请本机打开设置时，设置界面在这么久之内连上来才翻到它的设置页。
const SHOW_DEVICE_WINDOW: Duration = Duration::from_secs(30);

struct EngineTask {
    handle: JoinHandle<Result<Infallible>>,
    /// 服务端的即时设置；客户端没有。
    live: Option<watch::Sender<Live>>,
    /// 客户端的命令；服务端没有。
    client: Option<mpsc::UnboundedSender<ClientCommand>>,
}

struct Pending {
    prompt: PairingPrompt,
    reply: oneshot::Sender<bool>,
}

/// 一台连着的设备。
struct LinkInfo {
    fingerprint: Fingerprint,
    peer: PeerRef,
    /// 连上的时刻（Unix 毫秒）。
    since: i64,
    rtt: Rtt,
}

pub(crate) struct Runtime<H: Host> {
    store: Store,
    host: H,
    config: Config,
    name: String,
    fingerprint: Fingerprint,
    events: Events,
    asks: mpsc::UnboundedSender<PairAsk>,
    /// 文件传输。引擎重启后仍保留，断线重连后接着传。
    files: Files,
    /// 边缘投递胶囊。
    edge: EdgeDrop,

    paused: bool,
    engine: Option<EngineTask>,
    retry_at: Option<Instant>,
    problem: Option<Problem>,

    status: LinkStatus,
    /// 服务端正在监听的端口。
    port: Option<u16>,
    /// 连着的设备，按连上的先后。
    links: Vec<LinkInfo>,
    /// 光标所在的那台；客户端上，光标不在本机时是服务端。
    cursor: Option<Fingerprint>,
    /// 摘要里的那台，也是 `send_files` 不指定时发给的那台：光标最近去过的，否则最近连上的。
    focus: Option<Fingerprint>,
    clipboard: Option<ClipboardSync>,
    /// 最近一次剪贴板同步的对方。界面上的名字按它现在的名字显示（中途可能改了名）。
    clipboard_peer: Option<Fingerprint>,
    /// 剪贴板单项的大小上限，交给引擎共用，改设置时立即生效。
    clip_limit: ClipLimit,
    /// 最近一次同步的剪贴板图片存在磁盘上的那份，给设置界面预览。
    clip_preview: Option<PathBuf>,
    peers: PeerList,
    peers_read: Instant,
    last_seen: HashMap<Fingerprint, i64>,
    found: Vec<FoundServer>,
    autostart: bool,

    log: Log,
    activity: Activity,
    last_second: Instant,

    clients: BTreeMap<u64, mpsc::Sender<Notice>>,
    next_client: u64,
    /// 界面或 `quit` 命令请后台退出了。
    quit_requested: bool,
    pending: BTreeMap<u64, Pending>,
    next_prompt: u64,
    dirty: bool,
    /// 上次要打开设置界面的时刻，界面连上时据此记下启动用了多久。
    settings_opened: Option<Instant>,
    /// 本次运行中连上过的设备是什么系统。
    platforms: HashMap<Fingerprint, String>,
    /// 另一台设备请本机打开设置：等设置界面连上后翻到它（指纹的十六进制）的设置页。
    show_device: Option<(String, Instant)>,
    /// 客户端：等邀请的端口。
    invite_port: Option<u16>,
    /// 本机系统和版本，启动时问一次。
    system: String,
    /// 详细记录从什么时候开着（Unix 毫秒）。
    verbose_since: Option<i64>,
    /// 最近导出的诊断文件。
    last_export: Option<PathBuf>,
    /// 实际用的界面语言（托盘、屏幕边缘的胶囊）。
    lang: Lang,
    /// 版本更新。
    updates: Updates<H::Updater>,
    /// 本次运行中连上过的设备告诉的版本。
    versions: HashMap<Fingerprint, String>,
    /// 启动时的提示（例如“已更新到 …”），等第一个界面连上来再发。
    startup_toast: Option<(Phrase, String)>,
}

impl<H: Host> Runtime<H> {
    pub(crate) fn new(
        store: Store,
        host: H,
        events: Events,
        asks: mpsc::UnboundedSender<PairAsk>,
        update_tx: mpsc::UnboundedSender<UpdateEvent>,
    ) -> Result<Self> {
        let config = store.config()?;
        let updates = Updates::new(
            host.updater(),
            host.update_source(),
            store.clone(),
            update_tx,
        );
        let name = resolve_name(&config, &host);
        let fingerprint = store.identity(&name)?.fingerprint();
        let peers = store.peers()?;
        let autostart = host.autostart();
        let system = host.system();
        let lang = config.language.resolve(host.system_language());
        let files = Files::new(&store, events.clone());
        let edge = EdgeDrop::spawn(
            host.overlay(),
            files.clone(),
            accent_color(config.accent),
            lang,
        );
        let clip_limit = ClipLimit::new(config.clipboard_limit_bytes());
        // 上次留下的剪贴板图片预览。
        if let Err(e) = store.clear_blobs() {
            warn!("清理上次的剪贴板预览失败：{e}");
        }
        let mut runtime = Self {
            store,
            host,
            config,
            name,
            fingerprint,
            events,
            asks,
            files,
            edge,
            paused: false,
            engine: None,
            retry_at: None,
            problem: None,
            status: LinkStatus::Starting,
            port: None,
            links: Vec::new(),
            cursor: None,
            focus: None,
            clipboard: None,
            clipboard_peer: None,
            clip_limit,
            clip_preview: None,
            peers,
            peers_read: Instant::now(),
            last_seen: HashMap::new(),
            found: Vec::new(),
            autostart,
            log: Log::default(),
            activity: Activity::default(),
            last_second: Instant::now(),
            clients: BTreeMap::new(),
            next_client: 1,
            quit_requested: false,
            pending: BTreeMap::new(),
            next_prompt: 1,
            dirty: true,
            settings_opened: None,
            platforms: HashMap::new(),
            show_device: None,
            invite_port: None,
            system,
            verbose_since: None,
            last_export: None,
            lang,
            updates,
            versions: HashMap::new(),
            startup_toast: None,
        };
        let (role, role_key) = match runtime.config.role {
            Role::Server => ("键鼠在本机", "server"),
            Role::Client => ("由另一台电脑控制", "client"),
        };
        let phrase = Phrase::new("started")
            .arg("name", &runtime.name)
            .arg("role", role_key);
        runtime.log(
            LogKind::Link,
            LogIcon::Power,
            phrase,
            "ScreenFerry 已启动",
            format!("{} · {role}", mark::device(&runtime.name)),
        );
        let (outcomes, open_settings) = runtime.updates.startup();
        runtime.apply(outcomes);
        if open_settings {
            runtime.open_settings();
        }
        Ok(runtime)
    }

    pub(crate) fn next_client_id(&mut self) -> u64 {
        let id = self.next_client;
        self.next_client += 1;
        id
    }

    /// 有人请后台退出了（`quit` 命令）。
    pub(crate) fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// 退出前告诉所有界面一起关闭，并放开它们的发送队列：每个界面的发送任务写完这条后结束。
    /// 返回当时连着几个界面。
    pub(crate) fn farewell(&mut self) -> usize {
        self.broadcast(Notice::Quit);
        let count = self.clients.len();
        self.clients.clear();
        count
    }

    // ── 引擎 ─────────────────────────────────────

    pub(crate) fn start_engine(&mut self) {
        if self.paused || self.engine.is_some() {
            return;
        }
        self.retry_at = None;
        self.status = LinkStatus::Starting;
        self.dirty = true;
        let confirm = UiConfirm {
            asks: self.asks.clone(),
        };
        let started = match self.config.role {
            Role::Server => self.spawn_server(confirm),
            Role::Client => self.spawn_client(confirm),
        };
        if let Err(e) = started {
            self.fail(e);
        }
    }

    fn spawn_server(&mut self, confirm: UiConfirm) -> Result<()> {
        self.host.check_permissions(Role::Server)?;
        let capture = self.host.capture()?;
        let (live_tx, live_rx) = watch::channel(self.live());
        let config = ServerConfig {
            live: live_rx,
            name: self.name.clone(),
            clipboard: self.config.clipboard,
            clipboard_limit: self.clip_limit.clone(),
            platform: self.host.platform(),
            command_key: uses_command_key(),
            edge_drop: self.host.edge_drop(),
            store: self.store.clone(),
            announce: self.config.announce,
            events: self.events.clone(),
            files: self.files.clone(),
            edge: self.edge.feed(),
        };
        let (displays, clipboard) = (self.host.displays(), self.host.clipboard());
        let port = self.host.listen_port();
        let handle = tokio::spawn(async move {
            let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).await?;
            serve(listener, config, capture, displays, clipboard, confirm).await
        });
        self.engine = Some(EngineTask {
            handle,
            live: Some(live_tx),
            client: None,
        });
        Ok(())
    }

    fn spawn_client(&mut self, confirm: UiConfirm) -> Result<()> {
        self.host.check_permissions(Role::Client)?;
        let inject = self.host.inject()?;
        let server = match self.config.server.trim() {
            "" => ServerAddr::Discover,
            text => ServerAddr::Fixed(resolve(text)?),
        };
        let (commands_tx, commands) = mpsc::unbounded_channel();
        let config = ClientConfig {
            server,
            name: self.name.clone(),
            clipboard: self.config.clipboard,
            clipboard_limit: self.clip_limit.clone(),
            platform: self.host.platform(),
            command_key: uses_command_key(),
            edge_drop: self.host.edge_drop(),
            store: self.store.clone(),
            events: self.events.clone(),
            files: self.files.clone(),
            edge: self.edge.feed(),
            devices: self.config.devices.clone(),
            commands: Some(commands),
            invite_port: Some(self.host.invite_port()),
        };
        let (displays, clipboard) = (self.host.displays(), self.host.clipboard());
        let handle = tokio::spawn(connect(config, inject, displays, clipboard, confirm));
        self.engine = Some(EngineTask {
            handle,
            live: None,
            client: Some(commands_tx),
        });
        Ok(())
    }

    /// 停下引擎：丢弃它的任务，引擎随之归还鼠标控制、松开按键、断开连接。
    pub(crate) async fn stop_engine(&mut self) {
        if let Some(task) = self.engine.take() {
            task.handle.abort();
            let _ = task.handle.await;
        }
        self.edge.feed().reset();
        self.invite_port = None;
        self.drop_link();
        self.retry_at = None;
    }

    async fn restart_engine(&mut self) {
        self.stop_engine().await;
        self.start_engine();
    }

    /// 等待引擎自己结束（只会因为出错）。没有在运行的引擎时永远不返回。
    pub(crate) async fn engine_exit(&mut self) -> Result<Infallible> {
        let Some(task) = self.engine.as_mut() else {
            return std::future::pending().await;
        };
        let result = (&mut task.handle).await;
        self.engine = None;
        match result {
            Ok(result) => result,
            Err(e) => Err(Error::Io(std::io::Error::other(format!(
                "引擎意外退出：{e}"
            )))),
        }
    }

    pub(crate) fn on_engine_exit(&mut self, result: Result<Infallible>) {
        let Err(e) = result;
        warn!("引擎停止：{e}");
        self.drop_link();
        self.fail(e);
    }

    /// 引擎无法工作：记下问题，过一会儿再试。
    fn fail(&mut self, e: Error) {
        self.dirty = true;
        let (kind, delay) = match &e {
            Error::Platform(ferry_platform::Error::PermissionDenied(_)) => {
                (ProblemKind::Permission, RETRY_PERMISSION)
            }
            Error::Io(io) if io.kind() == ErrorKind::AddrInUse => {
                (ProblemKind::PortInUse, RETRY_OTHER)
            }
            Error::PairingRejected => {
                // 不是故障：用户或对方拒绝了配对，稍后照常寻找。
                self.status = LinkStatus::Searching;
                self.retry_at = Some(Instant::now() + RETRY_PAIRING);
                return;
            }
            _ => (ProblemKind::Other, RETRY_OTHER),
        };
        let message = match kind {
            ProblemKind::Permission => {
                "需要在系统设置的“隐私与安全性”中允许 ScreenFerry 控制键盘和鼠标".to_owned()
            }
            ProblemKind::PortInUse => format!(
                "端口 {} 被占用：可能还开着 Deskflow 或另一份 ScreenFerry",
                self.host.listen_port()
            ),
            ProblemKind::Other => e.to_string(),
        };
        let port = (kind == ProblemKind::PortInUse).then(|| self.host.listen_port());
        if self.problem.as_ref().is_none_or(|p| p.message != message) {
            let key = match kind {
                ProblemKind::Permission => "permission",
                ProblemKind::PortInUse => "port_in_use",
                ProblemKind::Other => "other",
            };
            let mut phrase = Phrase::new("cannot_share")
                .arg("problem", key)
                .arg("message", &message);
            if let Some(port) = port {
                phrase = phrase.arg("port", port);
            }
            self.log(
                LogKind::Link,
                LogIcon::Warning,
                phrase,
                "无法开始共享",
                message.clone(),
            );
        }
        self.problem = Some(Problem {
            kind,
            message,
            port,
        });
        self.status = LinkStatus::Stopped;
        self.retry_at = Some(Instant::now() + delay);
    }

    fn clear_problem(&mut self) {
        if self.problem.take().is_some() {
            self.dirty = true;
        }
    }

    fn live(&self) -> Live {
        Live {
            side: self.config.side,
            layout: self.config.layout.clone(),
            devices: self.config.devices.clone(),
            fallback: DeviceSettings::default(),
            mouse_dpi: self.config.mouse_dpi(),
        }
    }

    /// 连接全没了（引擎停下）：记下各台设备最后在线的时刻，清空与连接有关的状态。
    fn drop_link(&mut self) {
        if self.engine.is_none() {
            self.port = None;
        }
        let now = now_ms();
        for link in self.links.drain(..) {
            self.last_seen.insert(link.fingerprint, now);
        }
        self.cursor = None;
        self.focus = None;
        self.dirty = true;
    }

    fn link_mut(&mut self, fingerprint: &Fingerprint) -> Option<&mut LinkInfo> {
        self.links
            .iter_mut()
            .find(|l| l.fingerprint == *fingerprint)
    }

    /// 摘要里的那台连着的设备。
    fn focus_link(&self) -> Option<&LinkInfo> {
        self.focus
            .and_then(|f| self.links.iter().find(|l| l.fingerprint == f))
            .or(self.links.last())
    }

    /// 一台设备的名字：本机给它起过名字就用起的，否则连着时用它报的名字，再否则用配对时记下的。
    fn peer_name(&self, fingerprint: &Fingerprint) -> String {
        self.alias_of(fingerprint)
            .or_else(|| {
                self.links
                    .iter()
                    .find(|l| l.fingerprint == *fingerprint)
                    .map(|l| l.peer.name.clone())
            })
            .or_else(|| self.peers.get(fingerprint).map(|p| p.name.clone()))
            .unwrap_or_else(|| "对方".to_owned())
    }

    /// 本机给一台设备起的名字。
    fn alias_of(&self, fingerprint: &Fingerprint) -> Option<String> {
        let hex = fingerprint.to_hex();
        self.config
            .devices
            .iter()
            .find(|d| d.fingerprint == hex)
            .map(|d| d.alias.trim())
            .filter(|alias| !alias.is_empty())
            .map(str::to_owned)
    }

    /// 服务端：一台设备在本机哪一侧。
    fn side_of(&self, fingerprint: &Fingerprint) -> Side {
        let hex = fingerprint.to_hex();
        self.config
            .layout
            .iter()
            .find(|p| p.fingerprint == hex)
            .map_or(self.config.side, |p| p.side)
    }

    // ── 引擎事件 ─────────────────────────────────

    pub(crate) fn on_engine(&mut self, event: Event) {
        let progress = matches!(
            &event,
            Event::Transfer(TransferUpdate {
                state: TransferState::Running,
                ..
            })
        );
        if !matches!(event, Event::Rtt { .. }) && !progress {
            self.dirty = true;
        }
        match event {
            Event::Listening { port, announced } => {
                self.status = LinkStatus::Waiting;
                self.port = Some(port);
                self.clear_problem();
                let detail = if announced {
                    format!("等待另一台电脑连接 · 端口 {port}")
                } else {
                    format!("端口 {port} · 没有在局域网里广播")
                };
                let phrase = Phrase::new("listening")
                    .arg("port", port)
                    .arg("announced", announced);
                self.log(LogKind::Link, LogIcon::Power, phrase, "已开始共享", detail);
                self.invite_known(port);
            }
            Event::InviteListening { port } => self.invite_port = Some(port),
            Event::Dialed {
                address,
                invite,
                error,
            } => self.broadcast(Notice::Dialed {
                address,
                invite,
                error: error.map(ipc_dial_error),
            }),
            Event::Searching => {
                if self.links.is_empty() {
                    self.status = LinkStatus::Searching;
                }
                self.clear_problem();
            }
            Event::Found(list) => self.found = list,
            Event::Connecting { .. } => {
                if self.links.is_empty() {
                    self.status = LinkStatus::Connecting;
                }
            }
            Event::Connected {
                name,
                fingerprint,
                addr,
                platform,
            } => {
                if let Some(platform) = platform {
                    self.platforms.insert(fingerprint, platform);
                }
                self.status = LinkStatus::Connected;
                self.clear_problem();
                self.links.retain(|l| l.fingerprint != fingerprint);
                self.links.push(LinkInfo {
                    fingerprint,
                    peer: PeerRef {
                        name: name.clone(),
                        fingerprint: fingerprint.to_hex(),
                        short: fingerprint.short(),
                        addr: addr.ip().to_string(),
                    },
                    since: now_ms(),
                    rtt: Rtt::default(),
                });
                self.focus = Some(fingerprint);
                // 刚连上时光标总在服务端。
                if self.config.role == Role::Client {
                    self.cursor = Some(fingerprint);
                }
                self.refresh_peers();
                let name = self.alias_of(&fingerprint).unwrap_or(name);
                let phrase = Phrase::new("connected")
                    .arg("name", &name)
                    .arg("ip", addr.ip());
                self.log(
                    LogKind::Link,
                    LogIcon::Link,
                    phrase,
                    format!("已连接 {}", mark::device(&name)),
                    format!("TLS 1.3 · {}", addr.ip()),
                );
            }
            Event::Disconnected { peer, reason } => {
                let name = self.peer_name(&peer);
                self.links.retain(|l| l.fingerprint != peer);
                self.last_seen.insert(peer, now_ms());
                if self.cursor == Some(peer) {
                    self.cursor = None;
                }
                if self.focus == Some(peer) {
                    self.focus = self.links.last().map(|l| l.fingerprint);
                }
                self.status = match (self.links.is_empty(), self.config.role) {
                    (false, _) => LinkStatus::Connected,
                    (true, Role::Server) => LinkStatus::Waiting,
                    (true, Role::Client) => LinkStatus::Searching,
                };
                let mut phrase = Phrase::new("disconnected").arg("name", &name);
                if let Some(reason) = &reason {
                    phrase = phrase.arg("reason", reason);
                }
                self.log(
                    LogKind::Link,
                    LogIcon::Unlink,
                    phrase,
                    format!("与 {} 断开", mark::device(&name)),
                    reason.unwrap_or_else(|| "对方已断开".into()),
                );
            }
            Event::Rtt { peer, rtt } => {
                if let Some(link) = self.link_mut(&peer) {
                    link.rtt.record(rtt);
                }
            }
            Event::Transfer(update) => self.on_transfer(update),
            Event::OpenSettings { peer } => {
                let name = self.peer_name(&peer);
                self.log(
                    LogKind::Link,
                    LogIcon::Settings,
                    Phrase::new("peer_opened_settings").arg("name", &name),
                    format!("{} 请本机打开设置", mark::device(&name)),
                    "已翻到它的设置页",
                );
                // 已经开着的界面直接翻过去；没开的，打开后连上来时再翻。
                if self.clients.is_empty() {
                    self.show_device = Some((peer.to_hex(), Instant::now()));
                } else {
                    self.broadcast(Notice::ShowDevice {
                        fingerprint: peer.to_hex(),
                    });
                }
                self.open_settings();
            }
            Event::PeerVersion { peer, version } => {
                self.versions.insert(peer, version);
            }
            Event::PeerPlatform { peer, platform } => {
                self.platforms.insert(peer, platform);
            }
            Event::PeerSettings { peer, sent } => {
                let name = self.peer_name(&peer);
                if sent {
                    self.toast(
                        Phrase::new("peer_settings_opened").arg("name", &name),
                        format!("已在 {name} 上打开设置，把光标移过去就能改"),
                    );
                } else {
                    self.toast(
                        Phrase::new("peer_settings_unsupported").arg("name", &name),
                        format!("{name} 上的 ScreenFerry 版本较旧，请在那台电脑上打开设置"),
                    );
                }
            }
            Event::Cursor { here, hotkey, peer } => {
                self.cursor = (!here).then_some(peer);
                if !here {
                    self.focus = Some(peer);
                }
                let side = self.side_of(&peer);
                let name = self.peer_name(&peer);
                let peer = mark::device(&name).to_string();
                let (phrase, text, detail) = match (self.config.role, here) {
                    (Role::Server, false) => {
                        self.activity.switched();
                        (
                            Phrase::new("cursor_to_peer")
                                .arg("name", &name)
                                .arg("side", side_key(side)),
                            format!("光标切到 {peer}"),
                            format!("从本机{}边缘", side_label(side)),
                        )
                    }
                    (Role::Server, true) => (
                        Phrase::new("cursor_back").arg("hotkey", hotkey),
                        "光标回到本机".to_owned(),
                        if hotkey {
                            "快捷键 Ctrl + Option + Esc"
                        } else {
                            "从对方屏幕边缘"
                        }
                        .to_owned(),
                    ),
                    (Role::Client, true) => {
                        self.activity.switched();
                        (
                            Phrase::new("cursor_here").arg("name", &name),
                            "光标来到本机".to_owned(),
                            format!("从 {peer}"),
                        )
                    }
                    (Role::Client, false) => (
                        Phrase::new("cursor_returned").arg("name", &name),
                        format!("光标回到 {peer}"),
                        String::new(),
                    ),
                };
                self.log(LogKind::Cursor, LogIcon::Cursor, phrase, text, detail);
            }
            Event::Clipboard {
                peer,
                to_peer,
                summary,
            } => {
                self.clipboard_peer = Some(peer);
                let peer = self.peer_name(&peer);
                self.activity.clipped();
                let png_bytes = summary.image.as_ref().map(|i| i.png.len() as u64);
                let detail = match (&summary.image, summary.chars) {
                    (Some(image), _) => format!(
                        "图片 · {}×{} · {}",
                        image.width,
                        image.height,
                        size_text(image.png.len() as u64)
                    ),
                    (None, Some(n)) if summary.rich => format!("带格式的文本 · {n} 字"),
                    (None, Some(n)) => format!("文本 · {n} 字"),
                    (None, None) => size_text(summary.bytes),
                };
                let image = summary.image.map(|image| ClipboardImage {
                    width: image.width,
                    height: image.height,
                    path: self.save_preview(&image.png),
                });
                self.clipboard = Some(ClipboardSync {
                    to_peer,
                    peer: peer.clone(),
                    chars: summary.chars,
                    bytes: summary.bytes,
                    preview: summary.preview,
                    rich: summary.rich,
                    image,
                    at: now_ms(),
                });
                let text = if to_peer {
                    format!("剪贴板同步到 {}", mark::device(&peer))
                } else {
                    "剪贴板同步到本机".to_owned()
                };
                let mut phrase = Phrase::new("clipboard_synced")
                    .arg("name", &peer)
                    .arg("to_peer", to_peer)
                    .arg("bytes", summary.bytes);
                phrase = match (
                    &self.clipboard.as_ref().and_then(|c| c.image.as_ref()),
                    summary.chars,
                ) {
                    (Some(image), _) => phrase
                        .arg("content", "image")
                        .arg("bytes", png_bytes.unwrap_or(summary.bytes))
                        .arg("width", image.width)
                        .arg("height", image.height),
                    (None, Some(n)) => phrase
                        .arg("content", if summary.rich { "rich" } else { "text" })
                        .arg("chars", n),
                    (None, None) => phrase.arg("content", "data"),
                };
                self.log(LogKind::Clip, LogIcon::Clipboard, phrase, text, detail);
            }
            Event::ClipboardSkipped {
                peer,
                to_peer,
                format,
                bytes,
                limit,
            } => {
                let peer = self.peer_name(&peer);
                let what = match format {
                    ClipboardFormat::PNG => "图片",
                    ClipboardFormat::TEXT => "文本",
                    _ => "带格式的内容",
                };
                let text = if to_peer {
                    format!("剪贴板里的{what}太大，没有同步到 {}", mark::device(&peer))
                } else {
                    format!("{} 剪贴板里的{what}太大，没有同步过来", mark::device(&peer))
                };
                let detail = format!(
                    "{}，超过了 {} 的上限 · 可以在设置里调大",
                    size_text(bytes),
                    size_text(limit)
                );
                let format_key = match format {
                    ClipboardFormat::PNG => "image",
                    ClipboardFormat::TEXT => "text",
                    _ => "rich",
                };
                let phrase = Phrase::new("clipboard_skipped")
                    .arg("name", &peer)
                    .arg("to_peer", to_peer)
                    .arg("format", format_key)
                    .arg("bytes", bytes)
                    .arg("limit", limit);
                self.log(LogKind::Clip, LogIcon::Warning, phrase, text, detail);
            }
            Event::Paired { name, fingerprint } => {
                self.refresh_peers();
                let phrase = Phrase::new("paired")
                    .arg("name", &name)
                    .arg("short", fingerprint.short());
                self.log(
                    LogKind::Link,
                    LogIcon::Lock,
                    phrase.clone(),
                    format!("已与 {} 配对", mark::device(&name)),
                    format!("指纹 {}", fingerprint.short()),
                );
                self.toast(phrase, format!("已与 {name} 配对"));
            }
            Event::PairingRejected { name, by_peer } => {
                let text = if by_peer {
                    format!("{} 拒绝了配对", mark::device(&name))
                } else {
                    format!("已拒绝 {} 的配对请求", mark::device(&name))
                };
                let phrase = Phrase::new("pairing_rejected")
                    .arg("name", &name)
                    .arg("by_peer", by_peer);
                self.log(LogKind::Link, LogIcon::Warning, phrase, text, "");
            }
        }
    }

    // ── 配对 ─────────────────────────────────────

    pub(crate) fn on_pairing(&mut self, ask: PairAsk) {
        let id = self.next_prompt;
        self.next_prompt += 1;
        let request = &ask.request;
        let prompt = PairingPrompt {
            id,
            peer_name: request.peer_name.clone(),
            fingerprint: request.peer_fingerprint.to_hex(),
            short: request.peer_fingerprint.short(),
            code: request.code.to_string(),
        };
        let phrase = Phrase::new("pairing_request")
            .arg("name", &request.peer_name)
            .arg("code", request.code);
        self.log(
            LogKind::Link,
            LogIcon::Lock,
            phrase,
            format!("{} 请求配对", mark::device(&request.peer_name)),
            format!("配对码 {}", request.code),
        );
        self.broadcast(Notice::Pairing {
            prompt: prompt.clone(),
        });
        self.pending.insert(
            id,
            Pending {
                prompt,
                reply: ask.reply,
            },
        );
        // 没有界面在看：打开设置界面，用户才看得到配对码。
        if self.clients.is_empty() {
            self.host.open_settings();
        }
    }

    fn answer(&mut self, id: u64, accept: bool) {
        if let Some(pending) = self.pending.remove(&id) {
            let _ = pending.reply.send(accept);
            self.broadcast(Notice::PairingClosed { id });
        }
    }

    // ── 界面 ─────────────────────────────────────

    pub(crate) async fn on_ui(&mut self, event: UiEvent) {
        match event {
            UiEvent::Joined { id, outbox } => {
                let mut initial = vec![
                    Notice::History {
                        entries: self.log.entries(),
                    },
                    Notice::State {
                        state: Box::new(self.snapshot()),
                    },
                ];
                initial.extend(self.pending.values().map(|p| Notice::Pairing {
                    prompt: p.prompt.clone(),
                }));
                if let Some((fingerprint, at)) = self.show_device.take()
                    && at.elapsed() < SHOW_DEVICE_WINDOW
                {
                    initial.push(Notice::ShowDevice { fingerprint });
                }
                for notice in initial {
                    if outbox.try_send(notice).is_err() {
                        return;
                    }
                }
                if let Some((phrase, text)) = self.startup_toast.take() {
                    let _ = outbox.try_send(Notice::Toast {
                        text,
                        phrase: Some(phrase),
                    });
                }
                self.clients.insert(id, outbox);
                // 超过一分钟的不算：多半是那次没打开，这次是界面自己重连。
                if let Some(ms) = self
                    .settings_opened
                    .take()
                    .map(|at| at.elapsed().as_millis())
                    .filter(|&ms| ms < 60_000)
                {
                    info!(ms, "设置界面从打开到连上后台用了 {ms} 毫秒");
                }
            }
            UiEvent::Request { id, request } => self.on_request(id, request).await,
            UiEvent::Left { id } => {
                self.clients.remove(&id);
            }
        }
    }

    async fn on_request(&mut self, from: u64, request: Request) {
        match request {
            Request::Hello { .. } => {}
            Request::SetSharing { on } => self.set_sharing(on).await,
            Request::UpdateSettings { settings } => self.update_settings(settings).await,
            Request::SetDevice { device } => self.set_device(device).await,
            Request::OpenPeerSettings { fingerprint } => self.open_peer_settings(&fingerprint),
            Request::Forget { fingerprint } => self.forget(&fingerprint).await,
            Request::PairReply { id, accept } => self.answer(id, accept),
            Request::Connect { fingerprint } => self.connect_to(&fingerprint),
            Request::ConnectAddress { address } => self.connect_address(address),
            Request::Invite { address } => self.invite_address(address),
            Request::OpenPermissionSettings => self.host.open_permission_settings(),
            Request::ClearLog => {
                self.log.clear();
                self.broadcast(Notice::History {
                    entries: Vec::new(),
                });
            }
            Request::OpenSettings => self.open_settings(),
            Request::SendFiles { paths, to } => self.send_files(paths, to).await,
            Request::Quit => {
                info!("收到退出请求");
                self.quit_requested = true;
            }
            Request::ReadRunLog { since } => {
                let chunk = diagnostics::read(&self.store.log_file(), since);
                self.reply(
                    from,
                    Notice::RunLog {
                        lines: chunk.lines,
                        next: chunk.next,
                        fresh: chunk.fresh,
                        bytes: chunk.bytes,
                        lines_total: chunk.lines_total,
                    },
                );
            }
            Request::ExportDiagnostics => self.export_diagnostics(from),
            Request::CopyDiagnostics => {
                let text = self.diagnostics(true);
                self.reply(from, Notice::DiagnosticsSummary { text });
            }
            Request::RevealDiagnostics => match &self.last_export {
                Some(path) if path.exists() => self.host.reveal(path),
                _ => self.reply(
                    from,
                    Notice::Toast {
                        text: "诊断文件已经不在了，请重新导出".into(),
                        phrase: Some(Phrase::new("diagnostics_missing")),
                    },
                ),
            },
            Request::OpenLogsFolder => {
                let dir = self.store.logs_dir();
                let _ = fs::create_dir_all(&dir);
                self.host.open_folder(&dir);
            }
            Request::SetVerbose { on } => self.set_verbose(on),
            Request::CheckUpdate => self.updates.check(true),
            Request::DownloadUpdate => self.updates.download(),
            Request::CancelUpdate => self.updates.cancel(),
            Request::InstallUpdate => self.updates.install(!self.clients.is_empty()),
            Request::DismissUpdate { version } => self.updates.dismiss(version),
        }
        self.dirty = true;
    }

    /// 把剪贴板图片存一份给设置界面预览，换掉上一份。存不下来时界面只显示尺寸。
    fn save_preview(&mut self, png: &[u8]) -> Option<String> {
        let saved = match self.store.put_blob(png) {
            Ok(path) => path,
            Err(e) => {
                warn!("保存剪贴板图片的预览失败：{e}");
                return None;
            }
        };
        if let Some(old) = self.clip_preview.replace(saved.clone())
            && old != saved
            && let Err(e) = self.store.remove_blob(&old)
        {
            warn!("删除上一张剪贴板预览失败：{e}");
        }
        Some(saved.display().to_string())
    }

    /// 发给 `to`（证书指纹）；不指定时发给光标最近去过的那台，只连着一台时就是它。
    async fn send_files(&mut self, paths: Vec<String>, to: Option<String>) {
        let paths = paths.into_iter().map(PathBuf::from).collect();
        let target = match to {
            Some(hex) => hex.parse::<Fingerprint>().ok(),
            None => self.focus_link().map(|l| l.fingerprint),
        };
        let Some(peer) = target else {
            return self.toast(
                Phrase::new("not_connected"),
                Error::NotConnected.to_string(),
            );
        };
        if !self.config.device(&peer.to_hex()).files {
            let name = self.peer_name(&peer);
            return self.toast(
                Phrase::new("files_disabled").arg("name", &name),
                format!("已在设置里关掉了拖文件给 {name}"),
            );
        }
        match self.files.send(peer, paths).await {
            Ok(_) => {
                let name = self.peer_name(&peer);
                self.toast(
                    Phrase::new("sending").arg("name", &name),
                    format!("开始发给 {name}"),
                );
            }
            Err(e) => self.toast(Phrase::new("error").arg("message", &e), e.to_string()),
        }
    }

    /// 文件传输有了进展：开始、结束、中断都记进日志，过程中的进度不记。
    fn on_transfer(&mut self, update: TransferUpdate) {
        self.edge.transfer(update.clone());
        let name = self.peer_name(&update.peer);
        let peer = mark::device(&name).to_string();
        let what = if update.items > 1 {
            format!("{} 等 {} 项", mark::file(&update.name), update.items)
        } else {
            mark::file(&update.name).to_string()
        };
        let size = size_text(update.bytes);
        let files = format!("{} 个文件 · {size}", update.files);
        // 界面按当前语言说：文件名、项数、文件数、字节数都带上。
        let base = |key: &str| {
            Phrase::new(key)
                .arg("name", &name)
                .arg("file", &update.name)
                .arg("items", update.items)
                .arg("files", update.files)
                .arg("bytes", update.bytes)
        };
        let (phrase, text, detail) = match (&update.state, update.outgoing) {
            (TransferState::Running, _) => return,
            (TransferState::Waiting, true) => (
                base("transfer_sending"),
                format!("发送 {what} 到 {peer}"),
                files,
            ),
            (TransferState::Waiting, false) => (
                base("transfer_receiving"),
                format!("{peer} 发来 {what}"),
                files,
            ),
            (TransferState::Done { .. }, true) => (
                base("transfer_sent"),
                format!("已发送 {what}"),
                format!("到 {peer} · {size}"),
            ),
            (TransferState::Done { saved }, false) => {
                let folder = saved.first().and_then(|p| p.parent());
                let place = folder.map_or_else(String::new, |p| format!("保存在 {}", p.display()));
                let mut phrase = base("transfer_received");
                if let Some(folder) = folder {
                    phrase = phrase.arg("folder", folder.display());
                }
                (phrase, format!("已收到 {what}"), place)
            }
            (TransferState::Interrupted, _) => (
                base("transfer_interrupted"),
                format!("传输中断：{what}"),
                "重新连上后接着传".into(),
            ),
            (TransferState::Cancelled { by_peer }, _) => (
                base("transfer_cancelled").arg("by_peer", by_peer),
                format!("已取消：{what}"),
                if *by_peer {
                    "对方取消了"
                } else {
                    "本机取消了"
                }
                .into(),
            ),
            (TransferState::Failed { reason }, _) => (
                base("transfer_failed").arg("reason", reason),
                format!("传输失败：{what}"),
                reason.clone(),
            ),
        };
        self.log(LogKind::File, LogIcon::File, phrase, text, detail);
    }

    fn open_settings(&mut self) {
        self.settings_opened = Some(Instant::now());
        self.host.open_settings();
    }

    pub(crate) async fn on_command(&mut self, command: Command) {
        match command {
            Command::SetSharing(on) => self.set_sharing(on).await,
            Command::OpenSettings => self.open_settings(),
            Command::InstallUpdate => self.updates.install(!self.clients.is_empty()),
            Command::Quit => {}
        }
    }

    // ── 版本更新 ─────────────────────────────────

    pub(crate) fn on_update(&mut self, event: UpdateEvent) {
        let outcomes = self.updates.on_event(event, self.config.auto_update);
        self.apply(outcomes);
        self.dirty = true;
    }

    /// 为安装更新退出时（macOS）交给 `run` 的安装。
    pub(crate) fn take_restart(
        &mut self,
    ) -> Option<(std::sync::Arc<H::Updater>, ferry_platform::Prepared)> {
        self.updates.take_restart()
    }

    fn apply(&mut self, outcomes: Vec<Outcome>) {
        for outcome in outcomes {
            match outcome {
                Outcome::Toast(phrase, text) if self.clients.is_empty() => {
                    self.startup_toast = Some((phrase, text));
                }
                Outcome::Toast(phrase, text) => self.toast(phrase, text),
                Outcome::Log {
                    warning,
                    phrase,
                    text,
                    detail,
                } => {
                    let icon = if warning {
                        LogIcon::Warning
                    } else {
                        LogIcon::Settings
                    };
                    self.log(LogKind::Link, icon, phrase, text, detail);
                }
                Outcome::Restart => {
                    info!("为安装新版本退出");
                    self.quit_requested = true;
                }
            }
        }
    }

    async fn set_sharing(&mut self, on: bool) {
        if on != self.paused {
            return;
        }
        self.paused = !on;
        self.problem = None;
        self.dirty = true;
        if on {
            self.start_engine();
            let phrase = Phrase::new("sharing_resumed");
            self.log(
                LogKind::Link,
                LogIcon::Play,
                phrase.clone(),
                "继续共享",
                "键鼠可以切到对方",
            );
            self.toast(phrase, "已继续共享");
        } else {
            self.stop_engine().await;
            self.status = LinkStatus::Paused;
            let phrase = Phrase::new("sharing_paused");
            self.log(
                LogKind::Link,
                LogIcon::Pause,
                phrase.clone(),
                "暂停共享",
                "键鼠留在本机",
            );
            self.toast(phrase, "已暂停共享");
        }
    }

    async fn update_settings(&mut self, patch: SettingsPatch) {
        let mut next = self.config.clone();
        // 需要重新握手才能生效的设置。
        let mut restart = false;
        if let Some(name) = patch.name {
            restart |= name != next.name;
            next.name = name;
        }
        let role_changed = patch.role.filter(|&role| role != next.role);
        if let Some(role) = role_changed {
            restart = true;
            next.role = role;
        }
        if let Some(announce) = patch.announce {
            restart |= announce != next.announce;
            next.announce = announce;
        }
        if let Some(clipboard) = patch.clipboard {
            restart |= clipboard != next.clipboard;
            next.clipboard = clipboard;
        }
        if let Some(mb) = patch.clipboard_limit_mb {
            let range = Config::CLIPBOARD_LIMIT_MB;
            next.clipboard_limit_mb = mb.clamp(*range.start(), *range.end());
        }
        if let Some(server) = patch.server {
            restart |= server != next.server;
            next.server = server;
        }
        let moved = patch.side.filter(|&side| side != next.side);
        if let Some(side) = patch.side {
            next.side = side;
        }
        let arranged = patch.layout.as_ref().is_some_and(|l| *l != next.layout);
        if let Some(layout) = patch.layout {
            next.layout = layout;
        }
        if let Some(dpi) = patch.mouse_dpi {
            let range = Config::MOUSE_DPI;
            next.mouse_dpi = dpi.clamp(*range.start(), *range.end());
        }
        if let Some(theme) = patch.theme {
            next.theme = theme;
        }
        if let Some(on) = patch.auto_update {
            next.auto_update = on;
        }
        if let Some(language) = patch.language {
            next.language = language;
            self.lang = language.resolve(self.host.system_language());
            self.edge.set_lang(self.lang);
        }
        if let Some(accent) = patch.accent {
            next.accent = accent;
            self.edge.set_accent(accent_color(accent));
        }
        if let Some(on) = patch.autostart {
            match self.host.set_autostart(on) {
                Ok(()) => self.autostart = self.host.autostart(),
                Err(e) => self.toast(
                    Phrase::new("autostart_failed").arg("message", &e),
                    format!("无法修改开机自启：{e}"),
                ),
            }
            self.dirty = true;
        }

        if next != self.config {
            if let Err(e) = self.store.save_config(&next) {
                self.toast(
                    Phrase::new("save_failed").arg("message", &e),
                    format!("保存设置失败：{e}"),
                );
                return;
            }
            self.config = next;
            self.clip_limit.set(self.config.clipboard_limit_bytes());
            self.dirty = true;
        }
        if arranged {
            self.log(
                LogKind::Link,
                LogIcon::Layout,
                Phrase::new("layout_arranged"),
                "调整了设备的位置",
                "布局已保存",
            );
        } else if let Some(side) = moved {
            let (name, peer) = match self.links.as_slice() {
                [only] => (
                    only.peer.name.clone(),
                    mark::device(&only.peer.name).to_string(),
                ),
                _ => (String::new(), "其他设备".to_owned()),
            };
            let phrase = Phrase::new("layout_moved")
                .arg("name", &name)
                .arg("side", side_key(side));
            self.log(
                LogKind::Link,
                LogIcon::Layout,
                phrase,
                format!("{peer} 移到本机{}", side_label(side)),
                "布局已保存",
            );
        }
        if let Some(role) = role_changed {
            // 另一种角色用不上之前发现的服务端。
            self.found.clear();
            let (key, text, detail) = match role {
                Role::Server => (
                    "server",
                    "改为控制别的电脑",
                    "键盘鼠标在本机，等另一台电脑来连接",
                ),
                Role::Client => (
                    "client",
                    "改为由另一台电脑控制",
                    "在局域网里寻找键盘鼠标所在的那台",
                ),
            };
            let phrase = Phrase::new("role_changed").arg("role", key);
            self.log(LogKind::Link, LogIcon::Settings, phrase, text, detail);
        }
        if restart {
            self.name = resolve_name(&self.config, &self.host);
            self.restart_engine().await;
        } else {
            self.push_live();
        }
    }

    /// 换上一台设备的单独设置。服务端上立即生效；剪贴板开关要重新连接才生效，
    /// 客户端上对服务端的剪贴板、拖文件开关也一样。
    async fn set_device(&mut self, device: DeviceSettings) {
        let Ok(fingerprint) = device.fingerprint.parse::<Fingerprint>() else {
            return;
        };
        // 统一写成小写十六进制，免得同一台设备有两条。
        let device = DeviceSettings {
            fingerprint: fingerprint.to_hex(),
            ..device.clamped()
        };
        let before = self.config.device(&device.fingerprint);
        if before == device {
            return;
        }
        let mut next = self.config.clone();
        next.set_device(device.clone());
        if let Err(e) = self.store.save_config(&next) {
            return self.toast(
                Phrase::new("save_failed").arg("message", &e),
                format!("保存设置失败：{e}"),
            );
        }
        self.config = next;
        self.dirty = true;
        let linked = self.links.iter().any(|l| l.fingerprint == fingerprint);
        let reconnect = linked
            && (before.clipboard != device.clipboard
                || (self.config.role == Role::Client && before.files != device.files));
        if reconnect {
            self.restart_engine().await;
        } else {
            self.push_live();
        }
    }

    /// 客户端：请连着的服务端打开它的设置界面。
    fn open_peer_settings(&mut self, fingerprint: &str) {
        let Ok(fingerprint) = fingerprint.parse::<Fingerprint>() else {
            return;
        };
        let online = self.links.iter().any(|l| l.fingerprint == fingerprint);
        let commands = self.engine.as_ref().and_then(|t| t.client.as_ref());
        match commands {
            Some(commands) if online => {
                let _ = commands.send(ClientCommand::OpenPeerSettings);
            }
            _ => {
                let name = self.peer_name(&fingerprint);
                self.toast(
                    Phrase::new("peer_offline").arg("name", &name),
                    format!("{name} 现在没连着"),
                );
            }
        }
    }

    /// 把服务端的即时设置（布局、每台设备的设置）交给正在运行的引擎。
    fn push_live(&self) {
        if let Some(live) = self.engine.as_ref().and_then(|t| t.live.as_ref()) {
            live.send_if_modified(|current| {
                let next = self.live();
                let changed = *current != next;
                *current = next;
                changed
            });
        }
    }

    async fn forget(&mut self, fingerprint: &str) {
        let Ok(fingerprint) = fingerprint.parse::<Fingerprint>() else {
            return;
        };
        let mut peers = match self.store.peers() {
            Ok(peers) => peers,
            Err(e) => {
                return self.toast(
                    Phrase::new("peers_read_failed").arg("message", &e),
                    format!("读取已配对设备失败：{e}"),
                );
            }
        };
        let Some(name) = peers.get(&fingerprint).map(|p| p.name.clone()) else {
            return;
        };
        peers.remove(&fingerprint);
        if let Err(e) = self.store.save_peers(&peers) {
            return self.toast(
                Phrase::new("remove_failed").arg("message", &e),
                format!("移除失败：{e}"),
            );
        }
        self.refresh_peers();
        // 它的位置安排和单独设置也不再需要。
        let hex = fingerprint.to_hex();
        if self.config.layout.iter().any(|p| p.fingerprint == hex)
            || self.config.devices.iter().any(|d| d.fingerprint == hex)
        {
            let mut next = self.config.clone();
            next.layout.retain(|p| p.fingerprint != hex);
            next.devices.retain(|d| d.fingerprint != hex);
            match self.store.save_config(&next) {
                Ok(()) => self.config = next,
                Err(e) => warn!("保存设置失败：{e}"),
            }
        }
        let phrase = Phrase::new("removed").arg("name", &name);
        self.log(
            LogKind::Link,
            LogIcon::Unlink,
            phrase.clone(),
            format!("已移除 {}", mark::device(&name)),
            "下次连接需要重新配对",
        );
        self.toast(phrase, format!("已移除 {name}"));
        // 正连着这台：断开它，它下次来要重新配对。
        if self.links.iter().any(|l| l.fingerprint == fingerprint) {
            self.restart_engine().await;
        }
    }

    /// 客户端：连接局域网里发现的一台服务端。只连这一次，不把地址写死。
    fn connect_to(&mut self, fingerprint: &str) {
        if self.config.role != Role::Client {
            return;
        }
        let Ok(fingerprint) = fingerprint.parse::<Fingerprint>() else {
            return;
        };
        let Some(addr) = self
            .found
            .iter()
            .find(|f| f.fingerprint == fingerprint)
            .and_then(|f| f.addrs.first().copied())
        else {
            return self.toast(Phrase::new("server_gone"), "这台电脑已经不在局域网里了");
        };
        self.dial(addr);
    }

    /// 客户端：按地址连接一台服务端。电脑名在后台解析，结果以 `dialed` 告诉界面。
    fn connect_address(&mut self, address: String) {
        if self.config.role != Role::Client {
            return self.broadcast(Notice::Dialed {
                address,
                invite: false,
                error: Some(IpcDialError::WrongRole),
            });
        }
        let Some(commands) = self.engine.as_ref().and_then(|t| t.client.clone()) else {
            self.toast(
                Phrase::new("paused_connect_later"),
                "已暂停共享，继续共享后再连接",
            );
            return self.broadcast(Notice::Dialed {
                address,
                invite: false,
                error: Some(IpcDialError::Unreachable),
            });
        };
        let events = self.events.clone();
        tokio::spawn(async move {
            match resolve_input(&address, DEFAULT_PORT).await {
                Some(addr) => {
                    let _ = commands.send(ClientCommand::Connect(addr));
                }
                None => events.emit(Event::Dialed {
                    address,
                    invite: false,
                    error: Some(DialError::BadAddress),
                }),
            }
        });
    }

    fn dial(&mut self, addr: SocketAddr) {
        match self.engine.as_ref().and_then(|t| t.client.as_ref()) {
            Some(commands) => {
                let _ = commands.send(ClientCommand::Connect(addr));
            }
            None => self.toast(
                Phrase::new("paused_connect_later"),
                "已暂停共享，继续共享后再连接",
            ),
        }
    }

    /// 服务端：邀请这个地址上的电脑连过来。结果以 `dialed` 告诉界面。
    fn invite_address(&mut self, address: String) {
        let refuse = |error| Notice::Dialed {
            address: address.clone(),
            invite: true,
            error: Some(error),
        };
        if self.config.role != Role::Server {
            return self.broadcast(refuse(IpcDialError::WrongRole));
        }
        let Some(port) = self.port else {
            self.toast(
                Phrase::new("not_sharing_invite_later"),
                "还没开始共享，开始后再邀请",
            );
            return self.broadcast(refuse(IpcDialError::Unreachable));
        };
        let (store, name, events) = (self.store.clone(), self.name.clone(), self.events.clone());
        tokio::spawn(async move {
            let error = match resolve_input(&address, INVITE_PORT).await {
                None => Some(DialError::BadAddress),
                Some(addr) => invite(&store, &name, addr, port)
                    .await
                    .err()
                    .map(|e| DialError::of(&e)),
            };
            events.emit(Event::Dialed {
                address,
                invite: true,
                error,
            });
        });
    }

    /// 服务端开始共享时，邀请上次连过、现在没连着的客户端：局域网发现不可靠时也能自己连上来。
    /// 不成功也不提示。
    fn invite_known(&self, port: u16) {
        for peer in self.peers.iter() {
            let Some(addr) = peer.addr else {
                continue;
            };
            if self.links.iter().any(|l| l.fingerprint == peer.fingerprint) {
                continue;
            }
            let to = SocketAddr::new(addr.ip(), INVITE_PORT);
            let (store, name) = (self.store.clone(), self.name.clone());
            tokio::spawn(async move {
                if invite(&store, &name, to, port).await.is_ok() {
                    info!(%to, "已邀请上次连过的电脑");
                }
            });
        }
    }

    // ── 定时 ─────────────────────────────────────

    /// 每 250 毫秒一次：结算每秒的统计、清理过期的配对请求、按时重试、推送状态。
    pub(crate) fn on_tick(&mut self) {
        let outcomes = self.updates.tick(self.config.auto_update);
        if !outcomes.is_empty() {
            self.apply(outcomes);
            self.dirty = true;
        }
        let now = Instant::now();
        if now.duration_since(self.last_second) >= Duration::from_secs(1) {
            self.last_second = now;
            for link in &mut self.links {
                if link.rtt.roll() {
                    self.dirty = true;
                }
            }
            if !self.links.is_empty() {
                self.activity.connected_for(1);
            }
        }

        // 引擎那边已经不等了（超时、对方拒绝、连接断开）的配对请求。
        let closed: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, p)| p.reply.is_closed())
            .map(|(&id, _)| id)
            .collect();
        for id in closed {
            self.pending.remove(&id);
            self.broadcast(Notice::PairingClosed { id });
        }

        if self.engine.is_none() && self.retry_at.is_some_and(|at| now >= at) {
            self.start_engine();
        }
        if now.duration_since(self.peers_read) >= PEERS_REFRESH {
            self.refresh_peers();
        }
        if self.dirty {
            self.dirty = false;
            if !self.clients.is_empty() {
                let state = Box::new(self.snapshot());
                self.broadcast(Notice::State { state });
            }
        }
    }

    fn refresh_peers(&mut self) {
        self.peers_read = Instant::now();
        match self.store.peers() {
            Ok(peers) if peers != self.peers => {
                self.peers = peers;
                self.dirty = true;
            }
            Ok(_) => {}
            Err(e) => warn!("读取已配对设备失败：{e}"),
        }
    }

    // ── 输出 ─────────────────────────────────────

    /// 记一条日志：`phrase` 给界面按当前语言说，`text`、`detail` 是中文（写进运行日志，旧版界面也显示它）。
    fn log(
        &mut self,
        kind: LogKind,
        icon: LogIcon,
        phrase: Phrase,
        text: impl Into<String>,
        detail: impl Into<String>,
    ) {
        // 设备名、文件名带着标记写进运行日志（导出时换成代号），界面上显示去掉标记的样子。
        let (text, detail) = (text.into(), detail.into());
        info!(target: "screenferry", "{text} {detail}");
        let entry = self
            .log
            .push(kind, icon, mark::strip(&text), mark::strip(&detail), phrase);
        self.broadcast(Notice::Log { entry });
    }

    /// 只发给界面 `id` 的通知。
    fn reply(&mut self, id: u64, notice: Notice) {
        if let Some(outbox) = self.clients.get(&id)
            && outbox.try_send(notice).is_err()
        {
            warn!(id, "设置界面没有及时接收，已断开");
            self.clients.remove(&id);
        }
    }

    // ── 运行日志与诊断文件 ───────────────────────

    fn set_verbose(&mut self, on: bool) {
        if on == self.verbose_since.is_some() {
            return;
        }
        self.host.set_verbose(on);
        self.verbose_since = on.then(now_ms);
        info!("详细记录已{}", if on { "打开" } else { "关闭" });
        self.dirty = true;
    }

    /// 把诊断文件存到“下载”文件夹，重名时加编号。
    fn export_diagnostics(&mut self, from: u64) {
        let text = self.diagnostics(false);
        let (_, stamp) = diagnostics::local_time(now_ms(), self.host.utc_offset());
        let dir = self.store.downloads_dir();
        let name = ferry_core::file_name::unique(&format!("ScreenFerry-诊断-{stamp}.txt"), |n| {
            dir.join(n).exists()
        });
        let path = dir.join(name);
        let saved = fs::create_dir_all(&dir).and_then(|()| fs::write(&path, &text));
        match saved {
            Ok(()) => {
                info!(path = %mark::path(&path), "诊断文件已导出");
                self.last_export = Some(path.clone());
                self.reply(
                    from,
                    Notice::DiagnosticsExported {
                        path: path.display().to_string(),
                        bytes: text.len() as u64,
                    },
                );
            }
            Err(e) => {
                warn!("导出诊断文件失败：{e}");
                self.reply(
                    from,
                    Notice::Toast {
                        text: format!("导出诊断文件失败：{e}"),
                        phrase: Some(Phrase::new("diagnostics_failed").arg("message", &e)),
                    },
                );
            }
        }
    }

    /// 诊断文件（`summary` 为真时是只带最后 200 行运行日志的摘要）。设备名、文件名、用户目录换成代号。
    fn diagnostics(&self, summary: bool) -> String {
        let state = self.snapshot();
        let home = ferry_store::home_dir();
        let mut redactor = Redactor::new(home.as_deref().and_then(Path::to_str));
        redactor.local(&state.local.name);
        let server = state.local.role == Role::Server;
        let on = |b: bool| if b { "开" } else { "关" };

        let (now, _) = diagnostics::local_time(now_ms(), self.host.utc_offset());
        let version = match option_env!("SCREENFERRY_COMMIT") {
            Some(commit) if !commit.is_empty() => {
                format!("{}（提交 {commit}）", state.local.version)
            }
            _ => state.local.version.clone(),
        };
        let mut local = format!(
            "本机 · {}",
            if server {
                "控制别的电脑"
            } else {
                "被控制"
            }
        );
        if let Some(port) = state.local.port {
            local.push_str(&format!(" · 监听 {port}"));
        }
        if server {
            local.push_str(&format!(" · 局域网广播 {}", on(state.settings.announce)));
        } else {
            match state.settings.server.trim() {
                "" => local.push_str(" · 自动寻找控制端"),
                fixed => local.push_str(&format!(" · 固定连接 {fixed}")),
            }
            if let Some(port) = state.local.invite_port {
                local.push_str(&format!(" · 等邀请 {port}"));
            }
        }
        let displays = match ferry_platform::Displays::displays(&self.host.displays()) {
            Ok(list) => {
                let each: Vec<String> = list
                    .iter()
                    .map(|d| {
                        format!(
                            "{}×{} @{}%{}",
                            d.bounds.width,
                            d.bounds.height,
                            d.scale_percent,
                            if d.primary { "（主）" } else { "" }
                        )
                    })
                    .collect();
                format!("{} 台：{}", list.len(), each.join(" · "))
            }
            Err(e) => format!("读不到：{e}"),
        };
        let permission = match self.host.check_permissions(state.local.role) {
            Ok(()) => "键鼠控制 已允许".to_owned(),
            Err(e) => format!("键鼠控制 未允许：{e}"),
        };
        let mut devices = format!("{} 台已配对", state.peers.len());
        for peer in &state.peers {
            let code = redactor.device(&[&peer.name, &peer.alias]);
            let platform = match peer.platform.as_str() {
                "macos" => "macOS",
                "windows" => "Windows",
                "" => "系统未知",
                other => other,
            };
            let mut row = format!(
                "{code}  {platform} · {}",
                if peer.online { "在线" } else { "离线" }
            );
            if !peer.addr.is_empty() {
                row.push_str(&format!(" · {}", peer.addr));
            }
            if let Some(side) = peer.side {
                row.push_str(&format!(" · 在本机{}", side_label(side)));
            }
            let preset = match self.config.device(&peer.fingerprint).modifiers {
                ferry_types::ModifierPreset::Habit => "修饰键按本机习惯",
                ferry_types::ModifierPreset::Plain => "修饰键原样",
                ferry_types::ModifierPreset::Custom => "修饰键自定义",
            };
            row.push_str(&format!(" · {preset}"));
            devices.push('\n');
            devices.push_str(&row);
        }
        let settings = format!(
            "共享 {} · 剪贴板 {}（上限 {} MB）· 开机自启 {}",
            on(state.sharing),
            on(state.settings.clipboard),
            state.settings.clipboard_limit_mb,
            on(state.settings.autostart)
        );
        let verbose = match self.verbose_since {
            Some(since) => format!("开着，已有 {} 分钟", (now_ms() - since) / 60_000),
            None => "关".to_owned(),
        };
        let mut header = vec![
            ("生成于", now),
            ("版本", version),
            (
                "系统",
                if self.system.is_empty() {
                    std::env::consts::OS.to_owned()
                } else {
                    self.system.clone()
                },
            ),
            ("本机", local),
            ("显示器", displays),
            ("权限", permission),
            ("设备", devices),
            ("设置", settings),
            ("详细记录", verbose),
        ];
        // 局域网里发现、还没配对的设备也可能出现在日志里。
        for found in &state.found {
            redactor.device(&[&found.name]);
        }
        if let Some(problem) = &state.problem {
            header.push(("问题", redactor.line(&problem.message)));
        }
        let title = if summary {
            "ScreenFerry 诊断摘要"
        } else {
            "ScreenFerry 诊断文件"
        };
        let previous = self.store.previous_log_file();
        let current = self.store.log_file();
        diagnostics::report(
            title,
            &header,
            &[previous.as_path(), current.as_path()],
            &mut redactor,
            summary,
        )
    }

    /// 给所有界面一句提示：`phrase` 按当前语言说，`text` 是中文。
    fn toast(&mut self, phrase: Phrase, text: impl Into<String>) {
        self.broadcast(Notice::Toast {
            text: text.into(),
            phrase: Some(phrase),
        });
    }

    /// 发给所有界面。某个界面积压太多就断开它，它重连后会重新同步。
    fn broadcast(&mut self, notice: Notice) {
        self.clients
            .retain(|id, outbox| match outbox.try_send(notice.clone()) {
                Ok(()) => true,
                Err(_) => {
                    warn!(id, "设置界面没有及时接收，已断开");
                    false
                }
            });
    }

    pub(crate) fn summary(&self) -> Summary {
        let lang = self.lang;
        Summary {
            sharing: !self.paused,
            connected_to: (!self.links.is_empty()).then(|| {
                self.links
                    .iter()
                    .map(|l| self.peer_name(&l.fingerprint))
                    .collect::<Vec<_>>()
                    .join(lang.pick("、", ", "))
            }),
            problem: self.problem.as_ref().map(|p| match (p.kind, lang) {
                (ProblemKind::Permission, Lang::En) => {
                    "Allow ScreenFerry to control the keyboard and mouse in System Settings"
                        .to_owned()
                }
                (ProblemKind::PortInUse, Lang::En) => format!(
                    "Port {} is in use: another copy of ScreenFerry may be running",
                    p.port.unwrap_or_default()
                ),
                _ => p.message.clone(),
            }),
            lang,
            update: self.updates.ready_version(),
        }
    }

    pub(crate) fn snapshot(&self) -> State {
        let focus = self.focus_link();
        let server = self.config.role == Role::Server;
        State {
            local: Local {
                name: self.name.clone(),
                fingerprint: self.fingerprint.to_hex(),
                short: self.fingerprint.short(),
                role: self.config.role,
                data_dir: self.store.root().display().to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                port: self.port,
                platform: std::env::consts::OS.to_owned(),
                natural_scrolling: self.host.natural_scrolling(),
                addresses: ferry_net::local_addresses()
                    .into_iter()
                    .map(|ip| ip.to_string())
                    .collect(),
                invite_port: self.invite_port,
                system: self.system.clone(),
                verbose_since: self.verbose_since,
            },
            sharing: !self.paused,
            problem: self.problem.clone(),
            link: Link {
                status: self.status,
                peer: focus.map(|l| l.peer.clone()),
                since: focus.map(|l| l.since),
                cursor_on_peer: self.cursor.is_some(),
                rtt_ms: focus.map_or_else(Vec::new, |l| l.rtt.history()),
            },
            clipboard: self.clipboard.clone().map(|mut c| {
                if let Some(peer) = &self.clipboard_peer {
                    c.peer = self.peer_name(peer);
                }
                c
            }),
            peers: self
                .peers
                .iter()
                .map(|p| PeerInfo {
                    name: p.name.clone(),
                    fingerprint: p.fingerprint.to_hex(),
                    short: p.fingerprint.short(),
                    online: self.links.iter().any(|l| l.fingerprint == p.fingerprint),
                    last_seen: self.last_seen.get(&p.fingerprint).copied(),
                    side: server.then(|| self.side_of(&p.fingerprint)),
                    cursor: self.cursor == Some(p.fingerprint),
                    alias: self.alias_of(&p.fingerprint).unwrap_or_default(),
                    platform: self
                        .platforms
                        .get(&p.fingerprint)
                        .cloned()
                        .unwrap_or_default(),
                    addr: p.addr.map(addr_text).unwrap_or_default(),
                    version: self
                        .versions
                        .get(&p.fingerprint)
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect(),
            found: self
                .found
                .iter()
                .map(|f| FoundDevice {
                    name: f.name.clone(),
                    fingerprint: f.fingerprint.to_hex(),
                    short: f.fingerprint.short(),
                    addr: f
                        .addrs
                        .first()
                        .map(|a| a.ip().to_string())
                        .unwrap_or_default(),
                    paired: self.peers.contains(&f.fingerprint),
                })
                .collect(),
            settings: Settings {
                name: self.config.name.clone(),
                side: self.config.side,
                layout: self.config.layout.clone(),
                announce: self.config.announce,
                clipboard: self.config.clipboard,
                clipboard_limit_mb: self.config.clipboard_limit_mb,
                mouse_dpi: self.config.mouse_dpi(),
                server: self.config.server.clone(),
                autostart: self.autostart,
                theme: self.config.theme,
                accent: self.config.accent,
                language: self.config.language,
                auto_update: self.config.auto_update,
                devices: self.config.devices.clone(),
            },
            activity: self.activity.recent(),
            update: self.updates.info(!server && !self.links.is_empty()),
        }
    }
}

fn resolve_name<H: Host>(config: &Config, host: &H) -> String {
    match config.name.trim() {
        "" => host.device_name(),
        name => name.to_owned(),
    }
}

/// 解析 `IP`、`IP:端口`、`主机名` 或 `主机名:端口`，省略端口时用默认端口。
fn resolve(text: &str) -> Result<SocketAddr> {
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, DEFAULT_PORT));
    }
    let with_port = if text.contains(':') {
        text.to_owned()
    } else {
        format!("{text}:{DEFAULT_PORT}")
    };
    with_port
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .ok_or_else(|| Error::BadAddress(text.to_owned()))
}

/// 用户输入的地址：IP、IP:端口、电脑名或电脑名:端口；不写端口时用 `port`。解析不了时为 `None`。
async fn resolve_input(text: &str, port: u16) -> Option<SocketAddr> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Some(addr);
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Some(SocketAddr::new(ip, port));
    }
    let with_port = if text.contains(':') {
        text.to_owned()
    } else {
        format!("{text}:{port}")
    };
    let found = tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host(with_port))
        .await
        .ok()?
        .ok()?
        .collect::<Vec<_>>();
    found
        .iter()
        .find(|a| a.is_ipv4())
        .or(found.first())
        .copied()
}

/// 给用户看的地址：默认端口和不知道的端口（0）都不写。
fn addr_text(addr: SocketAddr) -> String {
    match addr.port() {
        0 | DEFAULT_PORT => addr.ip().to_string(),
        _ => addr.to_string(),
    }
}

fn ipc_dial_error(error: DialError) -> IpcDialError {
    match error {
        DialError::BadAddress => IpcDialError::BadAddress,
        DialError::Refused => IpcDialError::Refused,
        DialError::Unreachable => IpcDialError::Unreachable,
        DialError::NotScreenFerry => IpcDialError::NotScreenFerry,
        DialError::Incompatible => IpcDialError::Incompatible,
        DialError::Busy => IpcDialError::Busy,
    }
}

/// 方向写进 `Phrase` 的参数：`left`、`right`、`top`、`bottom`。
fn side_key(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
        Side::Top => "top",
        Side::Bottom => "bottom",
    }
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Left => "左侧",
        Side::Right => "右侧",
        Side::Top => "上方",
        Side::Bottom => "下方",
    }
}

/// 文件大小，给人看的写法。
fn size_text(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// 设置界面里第 `index` 个强调色（与 `app/lib/theme/tokens.dart` 中的浅色版一致）。
fn accent_color(index: u8) -> ferry_types::Rgba {
    const ACCENTS: [u32; 5] = [0x0A6CFF, 0x00796B, 0x6A4BD8, 0xC2410C, 0xC2185B];
    let rgb = ACCENTS
        .get(usize::from(index))
        .copied()
        .unwrap_or(ACCENTS[0]);
    ferry_types::Rgba::hex(rgb, 255)
}
