//! 常驻运行：用真实的本机通信当“设置界面”，平台换成 mock。

mod update;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferry_engine::resident::{self, Command, Host, UpdateSource};
use ferry_engine::{ClientConfig, EdgeFeed, Events, Files, ServerAddr, connect};
use ferry_ipc::{
    Connection, Endpoint, LinkStatus, Notice, PairingPrompt, ProblemKind, Request, SettingsPatch,
    State,
};
use ferry_platform::Error as PlatformError;
use ferry_platform_mock::{
    MockCapture, MockClipboard, MockDisplays, MockInject, MockOverlay, MockUpdater,
};
use ferry_types::{Config, Display, DisplayId, Rect, Role, Side, Theme};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};

use crate::support::{AutoConfirm, Device};

#[derive(Clone, Default)]
struct MockHost {
    /// 服务端监听的端口；默认 0，由系统分配，实际端口从状态里的 `local.port` 读。
    port: u16,
    deny_permissions: Arc<AtomicBool>,
    autostart: Arc<AtomicBool>,
    settings_opened: Arc<AtomicU32>,
    capture: MockCapture,
    inject: MockInject,
    verbose: Arc<AtomicBool>,
    /// 在访达里显示过的文件、打开过的文件夹，按先后。
    shown: Arc<Mutex<Vec<PathBuf>>>,
    updater: MockUpdater,
    /// 更新清单从哪里取；没设时指向一个连不上的地址，测试不碰外网。
    update_source: Option<UpdateSource>,
}

impl Host for MockHost {
    type Capture = MockCapture;
    type Inject = MockInject;
    type Displays = MockDisplays;
    type Clipboard = MockClipboard;
    type Overlay = MockOverlay;
    type Updater = MockUpdater;

    fn device_name(&self) -> String {
        "测试 Mac".into()
    }
    fn check_permissions(&self, _role: Role) -> ferry_platform::Result<()> {
        if self.deny_permissions.load(Ordering::SeqCst) {
            Err(PlatformError::PermissionDenied("测试中拒绝"))
        } else {
            Ok(())
        }
    }
    fn capture(&self) -> ferry_platform::Result<MockCapture> {
        Ok(self.capture.clone())
    }
    fn inject(&self) -> ferry_platform::Result<MockInject> {
        Ok(self.inject.clone())
    }
    fn displays(&self) -> MockDisplays {
        MockDisplays::new(vec![Display {
            id: DisplayId(1),
            bounds: Rect::new(0, 0, 1000, 800),
            scale_percent: 100,
            primary: true,
        }])
    }
    fn clipboard(&self) -> MockClipboard {
        MockClipboard::new()
    }

    fn overlay(&self) -> MockOverlay {
        MockOverlay::new()
    }
    fn updater(&self) -> MockUpdater {
        self.updater.clone()
    }
    fn update_source(&self) -> UpdateSource {
        self.update_source.clone().unwrap_or(UpdateSource {
            manifest_url: "http://127.0.0.1:9/update.json".into(),
            keys: Vec::new(),
        })
    }
    fn autostart(&self) -> bool {
        self.autostart.load(Ordering::SeqCst)
    }
    fn set_autostart(&self, on: bool) -> ferry_platform::Result<()> {
        self.autostart.store(on, Ordering::SeqCst);
        Ok(())
    }
    fn open_settings(&self) {
        self.settings_opened.fetch_add(1, Ordering::SeqCst);
    }
    fn open_permission_settings(&self) {}
    fn listen_port(&self) -> u16 {
        self.port
    }
    fn invite_port(&self) -> u16 {
        0
    }
    fn system(&self) -> String {
        "测试系统 1.0".into()
    }
    fn set_verbose(&self, on: bool) {
        self.verbose.store(on, Ordering::SeqCst);
    }
    fn reveal(&self, path: &Path) {
        self.shown.lock().unwrap().push(path.to_owned());
    }
    fn open_folder(&self, path: &Path) {
        self.shown.lock().unwrap().push(path.to_owned());
    }
}

/// 一份常驻后台，加上连着它的“设置界面”。
struct Daemon {
    device: Device,
    host: MockHost,
    handle: resident::Handle,
    task: JoinHandle<ferry_engine::Result<()>>,
}

impl Daemon {
    async fn start(host: MockHost) -> Self {
        Self::start_with(Device::new(), host).await
    }

    /// 在已经准备好的数据目录上启动。
    async fn start_with(device: Device, host: MockHost) -> Self {
        Self::start_configured(
            device,
            host,
            Config {
                role: Role::Server,
                announce: false,
                ..Config::default()
            },
        )
        .await
    }

    async fn start_configured(device: Device, host: MockHost, config: Config) -> Self {
        device.store.save_config(&config).unwrap();
        let (control, handle) = resident::control();
        let task = tokio::spawn(resident::run(device.store.clone(), host.clone(), control));
        let daemon = Self {
            device,
            host,
            handle,
            task,
        };
        daemon.endpoint().await;
        daemon
    }

    async fn endpoint(&self) -> Endpoint {
        let path = self.device.store.ipc_endpoint_path();
        for _ in 0..300 {
            if let Ok(Some(endpoint)) = Endpoint::load(&path) {
                return endpoint;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("后台没有写下 ipc.json");
    }

    async fn ui(&self) -> Ui {
        let conn = ferry_ipc::connect(&self.endpoint().await).await.unwrap();
        Ui { conn, state: None }
    }
}

/// 测试里的设置界面：记住最新的状态。
struct Ui {
    conn: Connection,
    state: Option<State>,
}

impl Ui {
    async fn send(&mut self, request: Request) {
        self.conn.send(&request).await.unwrap();
    }

    /// 读通知，直到 `check` 返回 `Some`。3 秒内等不到就失败。
    async fn wait<T>(&mut self, what: &str, mut check: impl FnMut(&Notice) -> Option<T>) -> T {
        let read = async {
            loop {
                let notice: Notice = self.conn.recv().await.unwrap().expect("后台断开了");
                if let Notice::State { state } = &notice {
                    self.state = Some((**state).clone());
                }
                if let Some(found) = check(&notice) {
                    return found;
                }
            }
        };
        timeout(Duration::from_secs(5), read)
            .await
            .unwrap_or_else(|_| panic!("等待超时：{what}"))
    }

    async fn state_where(&mut self, what: &str, pred: impl Fn(&State) -> bool) -> State {
        if let Some(state) = self.state.as_ref().filter(|s| pred(s)) {
            return state.clone();
        }
        self.wait(what, |n| match n {
            Notice::State { state } if pred(state) => Some((**state).clone()),
            _ => None,
        })
        .await
    }
}

#[tokio::test]
async fn the_ui_sees_the_state_and_can_pause_sharing() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;

    let state = ui
        .state_where("开始等待连接", |s| {
            s.link.status == LinkStatus::Waiting
        })
        .await;
    assert_eq!(state.local.name, "测试 Mac");
    assert_eq!(state.local.role, Role::Server);
    assert!(state.sharing);
    assert_eq!(state.local.fingerprint.len(), 64);

    ui.send(Request::SetSharing { on: false }).await;
    ui.wait("暂停的提示", |n| match n {
        Notice::Toast { text, phrase } if text == "已暂停共享" => {
            assert_eq!(
                phrase.as_ref().map(|p| p.key.as_str()),
                Some("sharing_paused")
            );
            Some(())
        }
        _ => None,
    })
    .await;
    ui.state_where("已暂停", |s| {
        !s.sharing && s.link.status == LinkStatus::Paused
    })
    .await;

    ui.send(Request::SetSharing { on: true }).await;
    ui.state_where("继续等待连接", |s| {
        s.sharing && s.link.status == LinkStatus::Waiting
    })
    .await;
    daemon.task.abort();
}

#[tokio::test]
async fn settings_are_saved_and_reported_back() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    ui.send(Request::UpdateSettings {
        settings: SettingsPatch {
            side: Some(Side::Left),
            theme: Some(Theme::Dark),
            accent: Some(3),
            autostart: Some(true),
            ..SettingsPatch::default()
        },
    })
    .await;
    let state = ui
        .state_where("设置已生效", |s| s.settings.side == Side::Left)
        .await;
    assert_eq!(state.settings.theme, Theme::Dark);
    assert_eq!(state.settings.accent, 3);
    assert!(state.settings.autostart, "开机自启以系统中的登记为准");

    let saved = daemon.device.store.config().unwrap();
    assert_eq!(saved.side, Side::Left);
    assert_eq!(saved.theme, Theme::Dark);
    daemon.task.abort();
}

#[tokio::test]
async fn device_settings_are_saved_and_reported_back() {
    use ferry_types::DeviceSettings;

    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    let fingerprint = "AB".repeat(32);
    let mut device = DeviceSettings::new(fingerprint.clone());
    device.alias = "书房的电脑".into();
    device.reverse_vertical = true;
    device.pointer_dpi = 50_000;
    ui.send(Request::SetDevice { device }).await;
    ui.send(Request::UpdateSettings {
        settings: SettingsPatch {
            mouse_dpi: Some(800),
            ..SettingsPatch::default()
        },
    })
    .await;
    let state = ui
        .state_where("设备设置已生效", |s| {
            !s.settings.devices.is_empty() && s.settings.mouse_dpi == 800
        })
        .await;
    let saved = &state.settings.devices[0];
    assert_eq!(
        saved.fingerprint,
        fingerprint.to_lowercase(),
        "统一写成小写"
    );
    assert_eq!(saved.alias, "书房的电脑");
    assert!(saved.reverse_vertical);
    assert_eq!(
        saved.pointer_dpi,
        *DeviceSettings::POINTER_DPI.end(),
        "超出范围的收进范围内"
    );

    let config = daemon.device.store.config().unwrap();
    assert_eq!(config.devices, state.settings.devices);
    assert_eq!(config.mouse_dpi, 800);

    // 恢复默认：这一条从配置里去掉。
    ui.send(Request::SetDevice {
        device: DeviceSettings::new(fingerprint),
    })
    .await;
    ui.state_where("恢复默认", |s| s.settings.devices.is_empty())
        .await;
    assert!(daemon.device.store.config().unwrap().devices.is_empty());
    daemon.task.abort();
}

#[tokio::test]
async fn first_pairing_is_confirmed_in_the_ui() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    let state = ui
        .state_where("开始等待连接", |s| {
            s.link.status == LinkStatus::Waiting
        })
        .await;
    let port = state.local.port.expect("等待连接时报告监听的端口");

    // 一台全新的客户端连过来，需要配对。
    let pc = Device::new();
    let pc_confirm = AutoConfirm::answering(true);
    let client = tokio::spawn(connect(
        ClientConfig {
            server: ServerAddr::Fixed(([127, 0, 0, 1], port).into()),
            name: "pc".into(),
            clipboard: true,

            clipboard_limit: Default::default(),
            command_key: false,
            store: pc.store.clone(),
            events: Events::none(),
            files: Files::new(&pc.store, Events::none()),
            edge: EdgeFeed::none(),
            devices: Vec::new(),
            commands: None,
            invite_port: None,
        },
        MockInject::new(),
        MockDisplays::new(vec![Display {
            id: DisplayId(1),
            bounds: Rect::new(0, 0, 1920, 1080),
            scale_percent: 150,
            primary: true,
        }]),
        MockClipboard::new(),
        pc_confirm.clone(),
    ));

    let prompt: PairingPrompt = ui
        .wait("配对请求", |n| match n {
            Notice::Pairing { prompt } => Some(prompt.clone()),
            _ => None,
        })
        .await;
    assert_eq!(prompt.peer_name, "pc");
    assert_eq!(prompt.code.len(), 7, "形如 123 456：{}", prompt.code);
    ui.send(Request::PairReply {
        id: prompt.id,
        accept: true,
    })
    .await;
    ui.wait("配对对话框关闭", |n| match n {
        Notice::PairingClosed { id } if *id == prompt.id => Some(()),
        _ => None,
    })
    .await;

    let state = ui
        .state_where("连上了", |s| s.link.status == LinkStatus::Connected)
        .await;
    let peer = state.link.peer.expect("连上时有对方");
    assert_eq!(peer.name, "pc");
    assert_eq!(peer.addr, "127.0.0.1");
    // 两边显示同一个配对码。
    assert_eq!(pc_confirm.requests()[0].code.to_string(), prompt.code);
    let state = ui
        .state_where(
            "已配对列表里有它且在线，并且知道它的版本",
            |s| {
                s.peers
                    .iter()
                    .any(|p| p.name == "pc" && p.online && p.version == env!("CARGO_PKG_VERSION"))
            },
        )
        .await;

    // 从界面移除它：断开，并从信任列表删掉。
    let fingerprint = state.peers[0].fingerprint.clone();
    ui.send(Request::Forget { fingerprint }).await;
    ui.state_where("移除后列表为空", |s| s.peers.is_empty())
        .await;
    assert!(daemon.device.store.peers().unwrap().is_empty());

    client.abort();
    daemon.task.abort();
}

#[tokio::test]
async fn input_keeps_working_after_both_settings_windows_disconnect() {
    use ferry_platform::CaptureEvent;
    use ferry_types::{InputEvent, KeyAction, KeyCode};

    use crate::support::{EDGE, eventually};

    let server = Daemon::start(MockHost::default()).await;
    let mut server_ui = server.ui().await;
    let port = server_ui
        .state_where("服务端已监听", |s| s.local.port.is_some())
        .await
        .local
        .port
        .unwrap();
    let client = Daemon::start_configured(
        Device::new(),
        MockHost::default(),
        Config {
            role: Role::Client,
            server: format!("127.0.0.1:{port}"),
            clipboard: false,
            ..Config::default()
        },
    )
    .await;
    let mut client_ui = client.ui().await;
    for ui in [&mut server_ui, &mut client_ui] {
        let id = ui
            .wait("配对请求", |n| match n {
                Notice::Pairing { prompt } => Some(prompt.id),
                _ => None,
            })
            .await;
        ui.send(Request::PairReply { id, accept: true }).await;
    }
    for ui in [&mut server_ui, &mut client_ui] {
        ui.state_where("已连接", |s| s.link.status == LinkStatus::Connected)
            .await;
    }
    drop(server_ui);
    drop(client_ui);
    // 给两端处理 IPC EOF 的时间，然后只通过键鼠和网络进行验证。
    sleep(Duration::from_millis(200)).await;
    eventually("没有设置界面也能切入被控端", || {
        if server.host.capture.is_grabbed() {
            return true;
        }
        server.host.capture.emit(CaptureEvent::Pointer {
            position: EDGE,
            dx: 1,
            dy: 0,
        });
        false
    })
    .await;
    let key = |action| InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action,
    };
    for action in [KeyAction::Down, KeyAction::Up] {
        server.host.capture.emit(CaptureEvent::Key {
            code: KeyCode::LEFT_SHIFT,
            action,
        });
        eventually("没有界面时仍注入键盘事件", || {
            client.host.inject.events().contains(&key(action))
        })
        .await;
    }
    server.host.capture.emit(CaptureEvent::Pointer {
        position: EDGE,
        dx: -50,
        dy: 0,
    });
    eventually("没有界面时仍能返回", || {
        !server.host.capture.is_grabbed()
    })
    .await;
    let mut reopened = client.ui().await;
    let state = reopened
        .state_where("重开界面仍是原来的连接", |s| {
            s.link.status == LinkStatus::Connected
        })
        .await;
    assert!(state.problem.is_none());
    server.task.abort();
    client.task.abort();
}

#[tokio::test]
async fn missing_permission_is_reported_and_retried() {
    let host = MockHost {
        deny_permissions: Arc::new(AtomicBool::new(true)),
        ..MockHost::default()
    };
    let daemon = Daemon::start(host.clone()).await;
    let mut ui = daemon.ui().await;
    let state = ui
        .state_where("报告缺少权限", |s| s.problem.is_some())
        .await;
    assert_eq!(state.problem.unwrap().kind, ProblemKind::Permission);
    assert_eq!(state.link.status, LinkStatus::Stopped);

    // 用户去系统设置里授权之后，几秒内自动恢复。
    host.deny_permissions.store(false, Ordering::SeqCst);
    ui.state_where("授权后开始等待连接", |s| {
        s.problem.is_none() && s.link.status == LinkStatus::Waiting
    })
    .await;
    daemon.task.abort();
}

#[tokio::test]
async fn a_second_launch_hands_over_to_the_running_one() {
    let daemon = Daemon::start(MockHost::default()).await;
    assert!(resident::hand_over_to_running(&daemon.device.store).await);
    let opened = daemon.host.settings_opened.clone();
    crate::support::eventually("请运行中的那份打开设置界面", || {
        opened.load(Ordering::SeqCst) == 1
    })
    .await;

    // 没有在运行的后台时，什么也不做。
    let other = Device::new();
    assert!(!resident::hand_over_to_running(&other.store).await);
    daemon.task.abort();
}

#[tokio::test]
async fn quitting_removes_the_endpoint_file() {
    let daemon = Daemon::start(MockHost::default()).await;
    let path = daemon.device.store.ipc_endpoint_path();
    assert!(path.exists());
    daemon.handle.send(Command::Quit);
    let result = timeout(Duration::from_secs(3), daemon.task).await.unwrap();
    assert!(result.unwrap().is_ok());
    assert!(!path.exists(), "退出后界面应当知道后台已经不在");
}

#[tokio::test]
async fn quitting_tells_the_ui_to_close_too() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    ui.state_where("首次状态", |_| true).await;
    daemon.handle.send(Command::Quit);
    ui.wait("后台要退出了", |n| (*n == Notice::Quit).then_some(()))
        .await;
    let result = timeout(Duration::from_secs(3), daemon.task).await.unwrap();
    assert!(result.unwrap().is_ok());
}

#[tokio::test]
async fn the_quit_command_stops_the_running_one_and_waits() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    ui.state_where("首次状态", |_| true).await;
    let path = daemon.device.store.ipc_endpoint_path();

    assert_eq!(resident::quit_running(&daemon.device.store).await, Ok(true));
    assert!(!path.exists(), "返回时后台已经退出");
    ui.wait("界面也收到了", |n| (*n == Notice::Quit).then_some(()))
        .await;
    let result = timeout(Duration::from_secs(3), daemon.task).await.unwrap();
    assert!(result.unwrap().is_ok());

    // 没有在运行的后台时，什么也不做。
    assert_eq!(
        resident::quit_running(&daemon.device.store).await,
        Ok(false)
    );
}

#[tokio::test]
async fn pairing_with_no_ui_open_opens_the_settings() {
    let host = MockHost::default();
    let daemon = Daemon::start(host.clone()).await;
    // 先用界面看一眼监听的端口，再关掉界面。
    let port = {
        let mut ui = daemon.ui().await;
        let state = ui
            .state_where("开始等待连接", |s| {
                s.link.status == LinkStatus::Waiting
            })
            .await;
        state.local.port.expect("等待连接时报告监听的端口")
    };
    // 等后台处理完界面断开，否则它会以为还有界面在看。
    sleep(Duration::from_millis(200)).await;
    let pc = Device::new();
    let client = tokio::spawn(connect(
        ClientConfig {
            server: ServerAddr::Fixed(([127, 0, 0, 1], port).into()),
            name: "pc".into(),
            clipboard: false,

            clipboard_limit: Default::default(),
            command_key: false,
            store: pc.store.clone(),
            events: Events::none(),
            files: Files::new(&pc.store, Events::none()),
            edge: EdgeFeed::none(),
            devices: Vec::new(),
            commands: None,
            invite_port: None,
        },
        MockInject::new(),
        MockDisplays::default(),
        MockClipboard::new(),
        AutoConfirm::answering(true),
    ));
    let opened = host.settings_opened.clone();
    crate::support::eventually("没有界面时打开设置界面", || {
        opened.load(Ordering::SeqCst) >= 1
    })
    .await;

    // 稍后打开的界面会收到仍在等待的配对请求。
    let mut ui = daemon.ui().await;
    ui.wait("补发配对请求", |n| match n {
        Notice::Pairing { .. } => Some(()),
        _ => None,
    })
    .await;
    client.abort();
    daemon.task.abort();
}

#[tokio::test]
async fn the_listening_port_is_reported_and_cleared_when_paused() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    let state = ui
        .state_where("开始等待连接", |s| {
            s.link.status == LinkStatus::Waiting
        })
        .await;
    assert!(
        state.local.port.is_some_and(|p| p != 0),
        "报告系统分配的实际端口"
    );

    ui.send(Request::SetSharing { on: false }).await;
    ui.state_where("暂停后不再监听", |s| s.local.port.is_none())
        .await;
    daemon.task.abort();
}

#[tokio::test]
async fn switching_the_role_restarts_as_the_other_side() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    ui.state_where("先以服务端等待连接", |s| {
        s.link.status == LinkStatus::Waiting && s.local.port.is_some()
    })
    .await;

    ui.send(Request::UpdateSettings {
        settings: SettingsPatch {
            role: Some(Role::Client),
            ..SettingsPatch::default()
        },
    })
    .await;
    let state = ui
        .state_where("改为客户端，不再监听", |s| {
            s.local.role == Role::Client && s.local.port.is_none()
        })
        .await;
    assert_ne!(state.link.status, LinkStatus::Waiting, "不再等别人来连");
    assert_eq!(state.local.platform, std::env::consts::OS);
    assert_eq!(daemon.device.store.config().unwrap().role, Role::Client);

    ui.send(Request::UpdateSettings {
        settings: SettingsPatch {
            role: Some(Role::Server),
            ..SettingsPatch::default()
        },
    })
    .await;
    ui.state_where("改回服务端后重新监听", |s| {
        s.local.role == Role::Server && s.local.port.is_some()
    })
    .await;
    daemon.task.abort();
}

// ── 运行日志与诊断文件 ─────────────────────────

/// 往运行日志里写几行：现在的格式，带标记的设备名、文件名和用户目录。
fn write_log(daemon: &Daemon, lines: &[String]) {
    let path = daemon.device.store.log_file();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
}

fn run_log(n: &Notice) -> Option<(Vec<ferry_ipc::RunLogLine>, u64, bool, u64)> {
    match n {
        Notice::RunLog {
            lines,
            next,
            fresh,
            lines_total,
            ..
        } => Some((lines.clone(), *next, *fresh, *lines_total)),
        _ => None,
    }
}

#[tokio::test]
async fn the_run_log_is_read_from_the_end_then_only_what_is_new() {
    use ferry_ipc::RunLogLevel;
    use ferry_types::mark;

    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    ui.state_where("首次状态", |_| true).await;
    write_log(
        &daemon,
        &[
            "   0.096268792s  INFO 以前的格式，没有时间".to_owned(),
            format!(
                "2026-11-02 06:12:04.437  INFO 已连接 {} TLS 1.3 · 192.168.2.18",
                mark::device("书房的电脑")
            ),
            "2026-11-02 06:20:02.775  WARN 连接中断：网络错误".to_owned(),
        ],
    );

    ui.send(Request::ReadRunLog { since: None }).await;
    let (lines, next, fresh, _) = ui.wait("运行日志", run_log).await;
    assert!(fresh);
    let ours: Vec<_> = lines.iter().rev().take(3).rev().collect();
    assert_eq!(ours[0].at, "");
    assert_eq!(ours[0].text, "以前的格式，没有时间");
    assert_eq!(ours[1].at, "06:12:04");
    assert_eq!(ours[1].level, RunLogLevel::Info);
    assert_eq!(
        ours[1].text, "已连接 书房的电脑 TLS 1.3 · 192.168.2.18",
        "界面上不带标记"
    );
    assert_eq!(ours[2].level, RunLogLevel::Warn);

    write_log(
        &daemon,
        &["2026-11-02 06:21:00.001 ERROR 心跳超时".to_owned()],
    );
    ui.send(Request::ReadRunLog { since: Some(next) }).await;
    let (lines, _, fresh, _) = ui.wait("新写的运行日志", run_log).await;
    assert!(!fresh, "接在后面");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, RunLogLevel::Error);
    assert_eq!(lines[0].text, "心跳超时");
    daemon.task.abort();
}

#[tokio::test]
async fn diagnostics_replace_names_files_and_the_home_folder_with_codes() {
    use ferry_types::mark;

    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    let state = ui.state_where("首次状态", |_| true).await;
    let local = state.local.name.clone();
    let home = ferry_store::home_dir().unwrap();
    let received = home.join("Downloads").join("证书.p12");
    write_log(
        &daemon,
        &[
            format!(
                "2026-11-02 06:12:03.104  INFO ScreenFerry 已启动 {}",
                mark::device(&local)
            ),
            format!(
                "2026-11-02 06:18:31.240  INFO 文件收好了 at={}",
                mark::path(&received)
            ),
            format!(
                "2026-11-02 06:19:00.000  WARN 读不到 {}",
                home.join("Desktop").display()
            ),
            format!("   0.096268792s  INFO 以前的版本写的 发送 私人.pdf 到 {local}"),
        ],
    );

    ui.send(Request::ExportDiagnostics).await;
    let path = ui
        .wait("导出诊断文件", |n| match n {
            Notice::DiagnosticsExported { path, .. } => Some(PathBuf::from(path)),
            _ => None,
        })
        .await;
    assert!(path.starts_with(daemon.device.store.downloads_dir()));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("ScreenFerry 诊断文件"));
    assert!(text.contains("测试系统 1.0"));
    assert!(text.contains("1 台：1000×800 @100%（主）"));
    assert!(text.contains("ScreenFerry 已启动 本机"));
    assert!(text.contains("文件 1.p12"));
    let home_text = home.display().to_string();
    assert!(text.contains("以前的版本写的 1 行没有带上"));
    for secret in [local.as_str(), "证书", "私人", home_text.as_str()] {
        assert!(!text.contains(secret), "诊断文件里不应出现 {secret}");
    }

    // 摘要只带最后 200 行。
    ui.send(Request::CopyDiagnostics).await;
    let summary = ui
        .wait("诊断摘要", |n| match n {
            Notice::DiagnosticsSummary { text } => Some(text.clone()),
            _ => None,
        })
        .await;
    assert!(summary.starts_with("ScreenFerry 诊断摘要"));
    assert!(summary.contains("文件 1.p12"));

    ui.send(Request::RevealDiagnostics).await;
    ui.send(Request::OpenLogsFolder).await;
    let shown = daemon.host.shown.clone();
    crate::support::eventually("在访达里显示", || shown.lock().unwrap().len() == 2).await;
    assert_eq!(shown.lock().unwrap()[0], path);
    assert_eq!(shown.lock().unwrap()[1], daemon.device.store.logs_dir());
    daemon.task.abort();
}

#[tokio::test]
async fn verbose_logging_can_be_switched_on_and_off() {
    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    ui.state_where("首次状态", |s| s.local.verbose_since.is_none())
        .await;
    assert_eq!(ui.state.as_ref().unwrap().local.system, "测试系统 1.0");

    ui.send(Request::SetVerbose { on: true }).await;
    ui.state_where("详细记录打开", |s| s.local.verbose_since.is_some())
        .await;
    assert!(daemon.host.verbose.load(Ordering::SeqCst));

    ui.send(Request::SetVerbose { on: false }).await;
    ui.state_where("详细记录关闭", |s| s.local.verbose_since.is_none())
        .await;
    assert!(!daemon.host.verbose.load(Ordering::SeqCst));
    daemon.task.abort();
}

#[tokio::test]
async fn the_language_is_saved_and_log_entries_say_what_happened() {
    use ferry_types::Language;

    let daemon = Daemon::start(MockHost::default()).await;
    let mut ui = daemon.ui().await;
    let entries = ui
        .wait("已有的日志", |n| match n {
            Notice::History { entries } => Some(entries.clone()),
            _ => None,
        })
        .await;
    let started = entries
        .iter()
        .find_map(|e| e.phrase.as_ref().filter(|p| p.key == "started"))
        .expect("启动那条日志带着 phrase");
    assert_eq!(started.args["role"], "server");
    assert!(!started.args["name"].is_empty());

    let state = ui.state_where("首次状态", |_| true).await;
    assert_eq!(state.settings.language, Language::System);
    ui.send(Request::UpdateSettings {
        settings: SettingsPatch {
            language: Some(Language::En),
            ..SettingsPatch::default()
        },
    })
    .await;
    ui.state_where("改成英文", |s| s.settings.language == Language::En)
        .await;
    assert_eq!(
        daemon.device.store.config().unwrap().language,
        Language::En,
        "存进 config.toml"
    );
    daemon.task.abort();
}
