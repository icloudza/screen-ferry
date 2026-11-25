//! 常驻运行：按 `config.toml` 启动引擎，经本机通信把状态推给设置界面、执行界面的命令。
//!
//! 暂停共享即停止引擎；改了需要重新握手的设置（设备名、剪贴板、广播、服务端地址）就重启引擎；
//! 屏幕方向和 ⌘/Ctrl 互换经 [`Live`](crate::Live) 即时生效。首次配对的配对码交给设置界面核对，
//! 没有界面连着时先打开它。见 ADR 0004。

mod confirm;
mod diagnostics;
mod record;
mod runtime;
mod ui;
mod update;

use std::path::{Path, PathBuf};
use std::time::Duration;

use ferry_ipc::{Endpoint, Listener};
use ferry_platform::{Clipboard, Displays, InputCapture, InputInject, Overlay};
use ferry_store::Store;
use ferry_types::Role;
use tokio::sync::{mpsc, watch};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use crate::error::Result;
use crate::events::Events;
use runtime::Runtime;
pub use update::{MANIFEST_URL, UpdateSource};

/// 状态推送与各种计时的节拍。界面最多每秒收到 4 次状态。
const TICK: Duration = Duration::from_millis(250);

/// 退出时最多等多久，让“后台要退出了”这条通知发到各个界面。
const FAREWELL: Duration = Duration::from_secs(1);

/// `quit` 命令最多等正在运行的后台多久。
const QUIT_WAIT: Duration = Duration::from_secs(10);

/// 由应用提供的本机能力：按需创建平台对象，以及只有应用才知道怎么做的事。
pub trait Host: Send + Sync + 'static {
    type Capture: InputCapture + 'static;
    type Inject: InputInject + 'static;
    type Displays: Displays + Sync + 'static;
    type Clipboard: Clipboard;
    type Overlay: Overlay + 'static;
    type Updater: ferry_platform::Updater + 'static;

    /// 本机缺省的设备名（计算机名）。
    fn device_name(&self) -> String;
    /// 本机系统，例如 `macos`、`windows`、`linux`。
    fn platform(&self) -> &'static str {
        "test"
    }
    /// 本机是否支持屏幕边缘投递。
    fn edge_drop(&self) -> bool {
        true
    }
    /// 检查本机作为 `role` 所需的权限。
    fn check_permissions(&self, role: Role) -> ferry_platform::Result<()>;
    fn capture(&self) -> ferry_platform::Result<Self::Capture>;
    fn inject(&self) -> ferry_platform::Result<Self::Inject>;
    fn displays(&self) -> Self::Displays;
    fn clipboard(&self) -> Self::Clipboard;
    /// 屏幕边缘的浮层（边缘投递胶囊）。
    fn overlay(&self) -> Self::Overlay;
    /// 自己更新（ADR 0014）。
    fn updater(&self) -> Self::Updater;
    /// 更新清单从哪里取、用哪些公钥校验。
    fn update_source(&self) -> UpdateSource {
        UpdateSource::official()
    }
    /// 是否已登记为登录时启动。
    fn autostart(&self) -> bool;
    fn set_autostart(&self, on: bool) -> ferry_platform::Result<()>;
    /// 打开（或前置）设置界面。
    fn open_settings(&self);
    /// 打开系统中授予键鼠控制权限的页面。
    fn open_permission_settings(&self);
    /// macOS：系统设置里开着“自然滚动”。其他系统为 `None`。
    fn natural_scrolling(&self) -> Option<bool> {
        None
    }
    /// 客户端等邀请的端口；0 表示由系统分配（测试用），实际端口见状态里的 `local.invite_port`。
    fn invite_port(&self) -> u16 {
        crate::INVITE_PORT
    }
    /// 服务端监听的端口；0 表示由系统分配（测试用），实际端口见状态里的 `local.port`。
    fn listen_port(&self) -> u16 {
        crate::DEFAULT_PORT
    }
    /// 系统的界面语言。设置里选了“跟随系统”时用它。
    fn system_language(&self) -> ferry_types::Lang {
        ferry_types::Lang::Zh
    }
    /// 本机系统和版本，例如“macOS 15.6 · arm64”。启动时问一次；不知道时为空。
    fn system(&self) -> String {
        String::new()
    }
    /// 本机时区与 UTC 相差的秒数，诊断文件里的时间换成本机时间时用。
    fn utc_offset(&self) -> i32 {
        0
    }
    /// 打开或关闭详细记录：运行日志多记调试信息。
    fn set_verbose(&self, on: bool) {
        let _ = on;
    }
    /// 在访达或资源管理器里显示并选中这个文件。
    fn reveal(&self, path: &Path) {
        let _ = path;
    }
    /// 用访达或资源管理器打开这个文件夹。
    fn open_folder(&self, path: &Path) {
        let _ = path;
    }
}

/// 托盘菜单等应用自身发来的命令。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    SetSharing(bool),
    OpenSettings,
    /// 安装已下载好的新版本。
    InstallUpdate,
    /// 停止引擎、删除 `ipc.json`，然后 [`run`] 返回。
    Quit,
}

/// 给托盘看的简要状态。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub sharing: bool,
    /// 已连接时是对方的设备名。
    pub connected_to: Option<String>,
    /// 妨碍工作的问题，已经按 `lang` 说好。
    pub problem: Option<String>,
    /// 托盘菜单用的语言。
    pub lang: ferry_types::Lang,
    /// 下载好、可以安装的新版本。
    pub update: Option<String>,
}

/// [`run`] 一侧：接收命令、发布简要状态。
pub struct Control {
    commands: mpsc::UnboundedReceiver<Command>,
    summary: watch::Sender<Summary>,
}

/// 应用一侧：发命令、看简要状态。可以克隆，例如托盘菜单的回调各持一份。
#[derive(Clone)]
pub struct Handle {
    commands: mpsc::UnboundedSender<Command>,
    summary: watch::Receiver<Summary>,
}

impl Handle {
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub fn summary(&self) -> watch::Receiver<Summary> {
        self.summary.clone()
    }
}

/// 新建一对控制端。
pub fn control() -> (Control, Handle) {
    let (commands_tx, commands) = mpsc::unbounded_channel();
    let (summary_tx, summary) = watch::channel(Summary::default());
    (
        Control {
            commands,
            summary: summary_tx,
        },
        Handle {
            commands: commands_tx,
            summary,
        },
    )
}

/// 常驻运行，直到收到 [`Command::Quit`] 或 [`Handle`] 全部被丢弃。
///
/// 启动时在 `store` 指定的位置写下 `ipc.json`，退出时删除。
/// 只有无法启动（例如数据目录不可用）时返回错误；引擎出的问题报告给界面，过一会儿自动重试。
pub async fn run<H: Host>(store: Store, host: H, mut control: Control) -> Result<()> {
    let (listener, endpoint) = Listener::bind().await?;
    let endpoint_path = store.ipc_endpoint_path();
    endpoint.save(&endpoint_path)?;
    let _endpoint = RemoveOnDrop {
        path: endpoint_path,
        pid: endpoint.pid,
    };
    info!(port = endpoint.port, "设置界面可以连接了");

    let (events, mut engine_events) = Events::channel();
    let (asks, mut pair_asks) = mpsc::unbounded_channel();
    let (ui_tx, mut ui_events) = mpsc::unbounded_channel();
    let (update_tx, mut update_events) = mpsc::unbounded_channel();
    let mut rt = Runtime::new(store, host, events, asks, update_tx)?;
    rt.start_engine();

    let mut tick = interval(TICK);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            incoming = listener.accept() => match incoming {
                Ok(incoming) => ui::accept(incoming, rt.next_client_id(), ui_tx.clone()),
                Err(e) => warn!("接受设置界面的连接失败：{e}"),
            },
            Some(event) = ui_events.recv() => {
                rt.on_ui(event).await;
                if rt.quit_requested() {
                    break;
                }
            }
            Some(event) = engine_events.recv() => rt.on_engine(event),
            Some(event) = update_events.recv() => {
                rt.on_update(event);
                if rt.quit_requested() {
                    break;
                }
            }
            Some(ask) = pair_asks.recv() => rt.on_pairing(ask),
            command = control.commands.recv() => match command {
                Some(Command::Quit) | None => break,
                Some(command) => rt.on_command(command).await,
            },
            exit = rt.engine_exit() => rt.on_engine_exit(exit),
            _ = tick.tick() => rt.on_tick(),
        }
        control.summary.send_if_modified(|current| {
            let next = rt.summary();
            let changed = *current != next;
            *current = next;
            changed
        });
    }
    info!("正在退出");
    let mut waiting = rt.farewell();
    rt.stop_engine().await;
    // 等每个界面的发送任务把最后一条通知写出去：写完后它结束，随即报告 Left。
    let flushed = async {
        while waiting > 0 {
            match ui_events.recv().await {
                Some(ui::UiEvent::Left { .. }) => waiting -= 1,
                Some(_) => {}
                None => break,
            }
        }
    };
    let _ = tokio::time::timeout(FAREWELL, flushed).await;
    // 为安装更新退出（macOS）：先删掉 ipc.json 让出位置，再启动新版本、等它起来。
    if let Some((updater, prepared)) = rt.take_restart() {
        let path = _endpoint.path.clone();
        drop(_endpoint);
        update::finish(updater, prepared, path).await;
    }
    Ok(())
}

/// 退出时删除 `ipc.json`，界面随即知道后台已经不在了。
struct RemoveOnDrop {
    path: PathBuf,
    pid: u32,
}

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        Endpoint::remove(&self.path, self.pid);
    }
}

/// 请正在运行的后台把这些文件发给它连着的对方，返回后台给的提示。
///
/// `paths` 须是绝对路径：后台的工作目录与调用方不同。
pub async fn send_via_running(store: &Store, paths: Vec<String>) -> Result<String, String> {
    use ferry_ipc::{Notice, Request};

    let endpoint = match Endpoint::load(&store.ipc_endpoint_path()) {
        Ok(Some(endpoint)) => endpoint,
        _ => return Err("ScreenFerry 没有在运行，请先打开它".into()),
    };
    let mut conn = ferry_ipc::connect(&endpoint)
        .await
        .map_err(|e| format!("连不上正在运行的 ScreenFerry：{e}"))?;
    conn.send(&Request::SendFiles { paths, to: None })
        .await
        .map_err(|e| format!("发不出请求：{e}"))?;
    // 后台展开大文件夹需要一点时间；先推来的状态快照和日志跳过，只等提示。
    let reply = async {
        loop {
            match conn.recv::<Notice>().await {
                Ok(Some(Notice::Toast { text, .. })) => return Ok(text),
                Ok(Some(_)) => {}
                _ => return Err("ScreenFerry 没有回复".to_owned()),
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(60), reply)
        .await
        .map_err(|_| "等 ScreenFerry 回复超时".to_owned())?
}

/// 请正在运行的后台退出，等它退出后返回 `Ok(true)`；没有在运行时返回 `Ok(false)`。
///
/// 安装程序升级、卸载前用它让后台放开程序文件。
pub async fn quit_running(store: &Store) -> Result<bool, String> {
    use ferry_ipc::{Notice, Request};

    let path = store.ipc_endpoint_path();
    let Ok(Some(endpoint)) = Endpoint::load(&path) else {
        return Ok(false);
    };
    // 连不上说明上次没有正常退出，留下的 ipc.json 已经没有意义。
    let Ok(mut conn) = ferry_ipc::connect(&endpoint).await else {
        return Ok(false);
    };
    conn.send(&Request::Quit)
        .await
        .map_err(|e| format!("发不出请求：{e}"))?;
    let finished = async {
        // 后台答应退出时先发 Quit，写完后放开连接。
        let mut agreed = false;
        while let Ok(Some(notice)) = conn.recv::<Notice>().await {
            agreed |= notice == Notice::Quit;
        }
        if !agreed {
            return Err(
                "正在运行的 ScreenFerry 版本较旧，不支持这个命令，请在托盘菜单里退出它".to_owned(),
            );
        }
        // 删掉 ipc.json 是退出前的最后一步。
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Ok(true)
    };
    tokio::time::timeout(QUIT_WAIT, finished)
        .await
        .map_err(|_| "等 ScreenFerry 退出超时".to_owned())?
}

/// 另一份后台是否已在运行：`ipc.json` 指向的端口能握手成功。
/// 在运行时请它打开设置界面并返回 `true`，当前进程应当退出。
pub async fn hand_over_to_running(store: &Store) -> bool {
    use ferry_ipc::{Notice, Request};

    let Ok(Some(endpoint)) = Endpoint::load(&store.ipc_endpoint_path()) else {
        return false;
    };
    let Ok(mut conn) = ferry_ipc::connect(&endpoint).await else {
        return false;
    };
    if conn.send(&Request::OpenSettings).await.is_err() {
        return false;
    }
    // 读掉对方随后推来的状态再断开：直接退出会让系统用 RST 断开连接，
    // 对方可能还没读到这条请求就收到了连接重置。
    let drain = async { while let Ok(Some(_)) = conn.recv::<Notice>().await {} };
    let _ = tokio::time::timeout(Duration::from_millis(300), drain).await;
    true
}
