//! 场景测试共用：在本机回环地址上跑一对真实的服务端与客户端，平台换成 mock。
//!
//! 连接是真实的 TLS，两端各有一个临时数据目录；配对确认由 [`AutoConfirm`] 自动回答。

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferry_core::Side;
use ferry_engine::{
    ClientCommand, ClientConfig, ClipLimit, ConfirmPairing, EdgeFeed, Error, Event, Events, Files,
    Live, PairingRequest, ServerAddr, ServerConfig, connect, serve,
};
use ferry_platform::CaptureEvent;
use ferry_platform_mock::{MockCapture, MockClipboard, MockDisplays, MockInject};
use ferry_store::Store;
use ferry_types::{Display, DisplayId, KeyAction, KeyCode, Point, Rect};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// 服务端本机 1000×800（100%），客户端 1920×1080（150%），客户端在右侧。
/// 服务端在右边缘 y = 400 处推出去一个单位，进入客户端的 (1, 540)：离边缘留出一个单位。
pub const EDGE: Point = Point::new(999, 400);
pub const ENTRY: Point = Point::new(1, 540);
/// 断开或按快捷键交还控制时，本机光标落在边缘往里一个单位。
pub const RETURN: Point = Point::new(998, 400);

/// 一台测试设备的数据目录。可以跨多个 `Rig` 复用，模拟重启后再连。
#[derive(Clone)]
pub struct Device {
    pub store: Store,
    /// 最后一个持有者丢弃时删除目录。
    _dir: Arc<TempDir>,
}

impl Device {
    pub fn new() -> Self {
        let dir = Arc::new(TempDir::new());
        let store = Store::open(dir.path()).unwrap();
        Self { store, _dir: dir }
    }
}

/// 自动回答配对确认，并记下每次的请求。
#[derive(Clone)]
pub struct AutoConfirm {
    answer: bool,
    requests: Arc<Mutex<Vec<PairingRequest>>>,
}

impl AutoConfirm {
    pub fn answering(answer: bool) -> Self {
        Self {
            answer,
            requests: Arc::default(),
        }
    }

    pub fn requests(&self) -> Vec<PairingRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ConfirmPairing for AutoConfirm {
    async fn confirm(&self, request: &PairingRequest) -> bool {
        self.requests.lock().unwrap().push(request.clone());
        self.answer
    }
}

/// 启动参数。默认：两台全新设备、都同意配对、都启动。
pub struct Setup {
    pub mac: Device,
    pub pc: Device,
    pub mac_clipboard: MockClipboard,
    pub pc_clipboard: MockClipboard,
    pub mac_confirm: AutoConfirm,
    pub pc_confirm: AutoConfirm,
    pub start_client: bool,
    pub pc_inject: Option<MockInject>,
    /// 服务端在局域网广播、客户端不填地址自己找。需要本机网络支持组播。
    pub discover: bool,
    /// 服务端可随时修改的设置；`None` 为固定的“右侧、互换 ⌘ 与 Ctrl”。
    pub live: Option<tokio::sync::watch::Receiver<Live>>,
    pub mac_events: Events,
    pub pc_events: Events,
    /// 服务端的边缘投递报告去处；默认不需要。
    pub edge: EdgeFeed,
    /// 服务端的文件传输；不给时按 `mac` 的数据目录新建。
    pub mac_files: Option<Files>,
    /// 客户端的边缘投递报告去处与文件传输；默认不需要、按 `pc` 的数据目录新建。
    pub pc_edge: EdgeFeed,
    pub pc_files: Option<Files>,
    /// 两端的快捷键是否以 Command 为主；默认服务端是 Mac、客户端是 PC。
    pub mac_command_key: bool,
    pub pc_command_key: bool,
    /// 两端的剪贴板单项上限；默认 32 MB。
    pub mac_clip_limit: ClipLimit,
    pub pc_clip_limit: ClipLimit,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            mac: Device::new(),
            pc: Device::new(),
            mac_clipboard: MockClipboard::new(),
            pc_clipboard: MockClipboard::new(),
            mac_confirm: AutoConfirm::answering(true),
            pc_confirm: AutoConfirm::answering(true),
            start_client: true,
            pc_inject: None,
            discover: false,
            live: None,
            mac_events: Events::none(),
            pc_events: Events::none(),
            edge: EdgeFeed::none(),
            mac_files: None,
            pc_edge: EdgeFeed::none(),
            pc_files: None,
            mac_command_key: true,
            pc_command_key: false,
            mac_clip_limit: ClipLimit::default(),
            pc_clip_limit: ClipLimit::default(),
        }
    }
}

pub struct Rig {
    pub addr: SocketAddr,
    pub capture: MockCapture,
    pub inject: MockInject,
    /// 两端的显示器，可以中途修改。
    pub mac_displays: MockDisplays,
    pub pc_displays: MockDisplays,
    /// 两端的文件传输。收到的文件在各自数据目录的 `received/` 里。
    pub mac_files: Files,
    pub pc_files: Files,
    pc_events: Events,
    /// 服务端（Mac）。
    pub mac: Device,
    /// 客户端（PC）。
    pub pc: Device,
    pub mac_clipboard: MockClipboard,
    pub pc_clipboard: MockClipboard,
    pub mac_confirm: AutoConfirm,
    pub pc_confirm: AutoConfirm,
    /// 服务端与客户端只会因出错而结束，任务的结果就是那个错误。
    pub server: JoinHandle<Error>,
    pub client: JoinHandle<Error>,
}

pub fn display(width: u32, height: u32, scale_percent: u16) -> Display {
    Display {
        id: DisplayId(1),
        bounds: Rect::new(0, 0, width, height),
        scale_percent,
        primary: true,
    }
}

impl Rig {
    pub async fn start() -> Self {
        Self::with(Setup::default()).await
    }

    /// 用给定的剪贴板启动，便于测试连接之前就有内容的情况。
    pub async fn with_clipboards(
        mac_clipboard: MockClipboard,
        pc_clipboard: MockClipboard,
    ) -> Self {
        Self::with(Setup {
            mac_clipboard,
            pc_clipboard,
            ..Setup::default()
        })
        .await
    }

    pub async fn with(setup: Setup) -> Self {
        // 广播出去的是本机的局域网地址，所以发现模式要监听所有网卡。
        let bind = if setup.discover {
            "0.0.0.0:0"
        } else {
            "127.0.0.1:0"
        };
        let listener = TcpListener::bind(bind).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let capture = MockCapture::new();
        let inject = setup.pc_inject.clone().unwrap_or_default();
        let mac_displays = MockDisplays::new(vec![display(1000, 800, 100)]);
        let pc_displays = MockDisplays::new(vec![display(1920, 1080, 150)]);
        let mac_files = setup
            .mac_files
            .clone()
            .unwrap_or_else(|| Files::new(&setup.mac.store, setup.mac_events.clone()));
        let pc_files = setup
            .pc_files
            .clone()
            .unwrap_or_else(|| Files::new(&setup.pc.store, setup.pc_events.clone()));

        let server = tokio::spawn({
            let capture = capture.clone();
            let displays = mac_displays.clone();
            let clipboard = setup.mac_clipboard.clone();
            let confirm = setup.mac_confirm.clone();
            let config = ServerConfig {
                live: setup.live.clone().unwrap_or_else(|| {
                    Live {
                        side: Side::Right,
                        layout: Vec::new(),
                        devices: Vec::new(),
                        fallback: ferry_types::DeviceSettings::default(),
                        mouse_dpi: 1600,
                    }
                    .fixed()
                }),
                name: "mac".into(),
                clipboard: true,

                clipboard_limit: setup.mac_clip_limit.clone(),
                command_key: setup.mac_command_key,
                store: setup.mac.store.clone(),
                announce: setup.discover,
                events: setup.mac_events.clone(),
                files: mac_files.clone(),
                edge: setup.edge.clone(),
            };
            async move {
                match serve(listener, config, capture, displays, clipboard, confirm).await {
                    Ok(never) => match never {},
                    Err(e) => e,
                }
            }
        });
        let client = if setup.start_client {
            Self::spawn_client(
                &setup,
                addr,
                inject.clone(),
                pc_displays.clone(),
                pc_files.clone(),
            )
        } else {
            tokio::spawn(std::future::pending())
        };
        Self {
            addr,
            capture,
            inject,
            mac_displays,
            pc_displays,
            mac_files,
            pc_files,
            pc_events: setup.pc_events.clone(),
            mac: setup.mac,
            pc: setup.pc,
            mac_clipboard: setup.mac_clipboard,
            pc_clipboard: setup.pc_clipboard,
            mac_confirm: setup.mac_confirm,
            pc_confirm: setup.pc_confirm,
            server,
            client,
        }
    }

    fn spawn_client(
        setup: &Setup,
        addr: SocketAddr,
        inject: MockInject,
        displays: MockDisplays,
        files: Files,
    ) -> JoinHandle<Error> {
        let clipboard = setup.pc_clipboard.clone();
        let confirm = setup.pc_confirm.clone();
        let config = ClientConfig {
            server: if setup.discover {
                ServerAddr::Discover
            } else {
                ServerAddr::Fixed(addr)
            },
            name: "pc".into(),
            clipboard: true,

            clipboard_limit: setup.pc_clip_limit.clone(),
            command_key: setup.pc_command_key,
            store: setup.pc.store.clone(),
            events: setup.pc_events.clone(),
            files,
            edge: setup.pc_edge.clone(),
            devices: Vec::new(),
            commands: None,
            invite_port: None,
        };
        tokio::spawn(async move {
            match connect(config, inject, displays, clipboard, confirm).await {
                Ok(never) => match never {},
                Err(e) => e,
            }
        })
    }

    /// 模拟断线：停掉客户端再重新启动，数据目录和文件传输都沿用原来的。
    pub fn restart_client(&mut self) {
        self.client.abort();
        let setup = Setup {
            pc: self.pc.clone(),
            pc_clipboard: self.pc_clipboard.clone(),
            pc_confirm: self.pc_confirm.clone(),
            pc_events: self.pc_events.clone(),
            ..Setup::default()
        };
        self.client = Self::spawn_client(
            &setup,
            self.addr,
            self.inject.clone(),
            self.pc_displays.clone(),
            self.pc_files.clone(),
        );
    }

    /// 停掉两端，留下数据目录供下一个 `Rig` 使用。
    pub fn stop(self) -> (Device, Device) {
        self.server.abort();
        self.client.abort();
        (self.mac, self.pc)
    }

    pub fn push(&self, dx: i32, dy: i32) {
        self.capture.emit(CaptureEvent::Pointer {
            position: EDGE,
            dx,
            dy,
        });
    }

    pub fn key(&self, code: KeyCode, action: KeyAction) {
        self.capture.emit(CaptureEvent::Key { code, action });
    }

    /// 在右边缘向外推，直到进入客户端屏幕，并且客户端已处理 Enter（把光标移到入口）。
    pub async fn enter(&self) {
        let before = self.inject.events().len();
        eventually("进入客户端屏幕", || {
            if self.capture.is_grabbed() {
                return true;
            }
            self.push(1, 0);
            false
        })
        .await;
        eventually("客户端处理 Enter", || {
            self.inject.events().len() > before
        })
        .await;
    }

    /// 从客户端屏幕左边缘推回本机。
    pub async fn leave(&self) {
        eventually("回到本机", || {
            if !self.capture.is_grabbed() {
                return true;
            }
            self.push(-50, 0);
            false
        })
        .await;
    }
}

/// 见 [`Rig::idle_client`]。
pub struct Idle {
    pub device: Device,
    pub events: tokio::sync::mpsc::UnboundedReceiver<Event>,
    pub commands: tokio::sync::mpsc::UnboundedSender<ClientCommand>,
}

impl Idle {
    /// 等到一个让 `pick` 返回 `Some` 的事件，最多 10 秒。
    pub async fn wait_for<T>(
        &mut self,
        what: &str,
        mut pick: impl FnMut(&Event) -> Option<T>,
    ) -> T {
        let found = async {
            while let Some(event) = self.events.recv().await {
                if let Some(found) = pick(&event) {
                    return found;
                }
            }
            panic!("{what}：事件通道关了");
        };
        tokio::time::timeout(Duration::from_secs(10), found)
            .await
            .unwrap_or_else(|_| panic!("{what}：10 秒内没等到"))
    }

    pub fn fingerprint(&self, name: &str) -> String {
        self.device
            .store
            .identity(name)
            .unwrap()
            .fingerprint()
            .to_hex()
    }
}

/// 同一台服务端上再连的一台客户端（1920×1080，150%），用来测同时连着几台设备。
pub struct Extra {
    pub name: String,
    pub device: Device,
    pub inject: MockInject,
    pub client: JoinHandle<Error>,
}

impl Extra {
    /// 它的证书指纹（十六进制）。连上之后才有。
    pub fn fingerprint(&self) -> String {
        self.device
            .store
            .identity(&self.name)
            .unwrap()
            .fingerprint()
            .to_hex()
    }
}

impl Rig {
    /// 再连上一台叫 `name` 的客户端，等它和服务端建好会话再返回。
    pub async fn add_client(&self, name: &str, edge: EdgeFeed) -> Extra {
        self.add_client_with(name, edge, None).await
    }

    /// 同 [`Rig::add_client`]，再交给它一个命令通道，测试借此像常驻后台一样给客户端下命令。
    pub async fn add_commanded_client(
        &self,
        name: &str,
    ) -> (Extra, tokio::sync::mpsc::UnboundedSender<ClientCommand>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let extra = self.add_client_with(name, EdgeFeed::none(), Some(rx)).await;
        (extra, tx)
    }

    async fn add_client_with(
        &self,
        name: &str,
        edge: EdgeFeed,
        commands: Option<tokio::sync::mpsc::UnboundedReceiver<ClientCommand>>,
    ) -> Extra {
        let device = Device::new();
        let (events, mut rx) = Events::channel();
        let files = Files::new(&device.store, events.clone());
        let inject = MockInject::new();
        let config = ClientConfig {
            server: ServerAddr::Fixed(self.addr),
            name: name.into(),
            clipboard: true,

            clipboard_limit: Default::default(),
            command_key: false,
            store: device.store.clone(),
            events,
            files,
            edge,
            devices: Vec::new(),
            commands,
            invite_port: None,
        };
        let client = tokio::spawn({
            let inject = inject.clone();
            let displays = MockDisplays::new(vec![display(1920, 1080, 150)]);
            async move {
                let confirm = AutoConfirm::answering(true);
                match connect(config, inject, displays, MockClipboard::new(), confirm).await {
                    Ok(never) => match never {},
                    Err(e) => e,
                }
            }
        });
        let connected = async {
            while let Some(event) = rx.recv().await {
                if matches!(event, Event::Connected { .. }) {
                    return;
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(5), connected)
            .await
            .expect("新的客户端 5 秒内没有连上");
        // 之后的事件不关心，免得积压。
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        let extra = Extra {
            name: name.into(),
            device,
            inject,
            client,
        };
        // 客户端报告连上时，服务端可能还没登记这个会话；等服务端也连上它的数据连接。
        let fingerprint = extra.fingerprint();
        eventually("服务端登记了新的客户端", || {
            self.mac_files
                .connected()
                .iter()
                .any(|f| f.to_hex() == fingerprint)
        })
        .await;
        extra
    }

    /// 一台还没连上服务端的客户端：它固定去连一个没人在等的地址（一直重试），等测试让它按地址连接，
    /// 或者邀请它。`invite` 为真时它在系统分配的端口上等邀请。
    pub fn idle_client(&self, name: &str, invite: bool) -> Idle {
        let device = Device::new();
        let (events, rx) = Events::channel();
        let (commands, commands_rx) = tokio::sync::mpsc::unbounded_channel();
        let config = ClientConfig {
            server: ServerAddr::Fixed(SocketAddr::from(([127, 0, 0, 1], 1))),
            name: name.into(),
            clipboard: true,
            clipboard_limit: Default::default(),
            command_key: false,
            store: device.store.clone(),
            events,
            files: Files::new(&device.store, Events::none()),
            edge: EdgeFeed::none(),
            devices: Vec::new(),
            commands: Some(commands_rx),
            invite_port: invite.then_some(0),
        };
        // 测试结束时运行时一停，它也就停了。
        tokio::spawn(async move {
            let confirm = AutoConfirm::answering(true);
            let _ = connect(
                config,
                MockInject::new(),
                MockDisplays::new(vec![display(1920, 1080, 150)]),
                MockClipboard::new(),
                confirm,
            )
            .await;
        });
        Idle {
            device,
            events: rx,
            commands,
        }
    }

    /// 主客户端 `pc` 的证书指纹（十六进制）。
    pub fn pc_fingerprint(&self) -> String {
        self.pc.store.identity("pc").unwrap().fingerprint().to_hex()
    }

    /// 本机光标停在 `(x, y)`，再朝外推 `(dx, dy)`。
    pub fn push_at(&self, x: i32, y: i32, dx: i32, dy: i32) {
        self.capture.emit(CaptureEvent::Pointer {
            position: Point::new(x, y),
            dx,
            dy,
        });
    }
}

/// 反复检查，直到条件成立；3 秒内不成立就判定失败。
pub async fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    for _ in 0..300 {
        if check() {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }
    panic!("等待超时：{what}");
}

/// 测试结束时删除的临时目录。
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ferry-engine-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 把事件收集到一个列表里，供断言。
pub fn collect() -> (Events, Arc<Mutex<Vec<Event>>>) {
    let (events, mut rx) = Events::channel();
    let seen = Arc::new(Mutex::new(Vec::new()));
    tokio::spawn({
        let seen = seen.clone();
        async move {
            while let Some(event) = rx.recv().await {
                seen.lock().unwrap().push(event);
            }
        }
    });
    (events, seen)
}

pub fn has(seen: &Mutex<Vec<Event>>, pred: impl Fn(&Event) -> bool) -> bool {
    seen.lock().unwrap().iter().any(pred)
}

/// 一段只有文件头像 PNG 的数据，后面补 `padding` 个字节。读尺寸只看文件头，同步时原样传过去。
pub fn fake_png(width: u32, height: u32, padding: usize) -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&width.to_be_bytes());
    png.extend_from_slice(&height.to_be_bytes());
    png.resize(png.len() + padding, 0xAB);
    png
}
