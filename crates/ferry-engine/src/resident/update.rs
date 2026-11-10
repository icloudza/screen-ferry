//! 版本更新：定时检查、后台下载、安装（ADR 0014）。
//!
//! - 自动检查开着时，启动两分钟后查一次，之后每天一次（时间带随机偏移）；有新版本就在后台下载好，
//!   由用户决定什么时候重启。关着时只在用户点“检查更新”时查，发现新版本也等用户点“下载”。
//! - 清单要通过签名校验、没过期、不比见过的旧；下载的文件大小和 SHA-256 要和清单一致。
//! - 安装交给平台：macOS 换好应用后由本进程退出、启动新版本，新版本 20 秒内没起来就换回旧版本；
//!   Windows 以管理员身份运行安装包，由安装包请本进程退出。
//! - 安装前在 `updates/state.toml` 记下“正在从哪个版本装到哪个版本”，新版本启动后据此提示。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ferry_ipc::{Endpoint, Phrase, UpdateError, UpdateInfo, UpdateRelease, UpdateStatus};
use ferry_net::Https;
use ferry_platform::{Prepared, Updater};
use ferry_store::{Installing, Store, UpdateState};
use ferry_update::{Asset, Release, Verdict, Version, judge};
use tokio::sync::mpsc;
use tracing::{info, warn};

/// 正式的更新清单：GitHub 上最新一个正式版里的 `update.json`（签名在 `update.json.minisig`）。
/// 走下载地址而不是 API，没有每小时 60 次的限制。
pub const MANIFEST_URL: &str =
    "https://github.com/icloudza/screen-ferry/releases/latest/download/update.json";

/// 启动后多久第一次检查。
const FIRST_CHECK: Duration = Duration::from_secs(120);
/// 之后多久检查一次，自动检查失败后多久再试。
const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);
const RETRY_AFTER: Duration = Duration::from_secs(3600);
/// Windows：安装包启动后这么久还没让本进程退出，就当没装成。
const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);
/// macOS：新版本启动后最多等它这么久。
const NEW_VERSION_WAIT: Duration = Duration::from_secs(20);
/// 清单和签名的大小上限。
const MANIFEST_LIMIT: u64 = 64 * 1024;
const SIGNATURE_LIMIT: u64 = 4 * 1024;

/// 本机的版本。
pub(crate) fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 0, 0))
}

/// 更新清单从哪里取、用哪些公钥校验。测试里换成本机的服务器和临时密钥。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateSource {
    pub manifest_url: String,
    pub keys: Vec<String>,
}

impl UpdateSource {
    /// 正式的清单和内置的发布公钥。
    pub fn official() -> Self {
        Self {
            manifest_url: MANIFEST_URL.to_owned(),
            keys: ferry_update::RELEASE_KEYS
                .iter()
                .map(|k| (*k).to_owned())
                .collect(),
        }
    }
}

/// 后台任务报回来的结果。
pub(crate) enum UpdateEvent {
    Checked(Result<Release, (UpdateError, String)>),
    Progress {
        done: u64,
        total: u64,
    },
    /// 下载的结果；`Err(None)` 是用户取消了。
    Downloaded(Result<PathBuf, Option<(UpdateError, String)>>),
    Prepared(Result<Prepared, String>),
    Launched(Result<(), ferry_platform::Error>),
}

/// 交给常驻运行去做的事：提示、记日志、为安装退出。
pub(crate) enum Outcome {
    Toast(Phrase, String),
    Log {
        warning: bool,
        phrase: Phrase,
        text: String,
        detail: String,
    },
    /// macOS：新应用已经换好，本进程该退出了，退出后由 [`finish`] 启动新版本。
    Restart,
}

pub(crate) struct Updates<U> {
    platform: Arc<U>,
    source: UpdateSource,
    store: Store,
    tx: mpsc::UnboundedSender<UpdateEvent>,
    current: Version,
    state: UpdateState,

    status: UpdateStatus,
    error: Option<(UpdateError, String)>,
    release: Option<Release>,
    package: Option<PathBuf>,
    done: u64,
    total: u64,
    checked_at: Option<i64>,
    /// 这次检查是用户点的：出错时要告诉用户。
    manual: bool,
    next_check: Instant,
    cancel: Arc<AtomicBool>,
    installing_since: Option<Instant>,
    /// macOS：换好了新应用，等本进程退出后启动它。
    pending_install: Option<Prepared>,
}

impl<U: Updater + 'static> Updates<U> {
    pub(crate) fn new(
        platform: U,
        source: UpdateSource,
        store: Store,
        tx: mpsc::UnboundedSender<UpdateEvent>,
    ) -> Self {
        let state = store.update_state();
        Self {
            platform: Arc::new(platform),
            source,
            store,
            tx,
            current: current_version(),
            state,
            status: UpdateStatus::Idle,
            error: None,
            release: None,
            package: None,
            done: 0,
            total: 0,
            checked_at: None,
            manual: false,
            next_check: Instant::now() + FIRST_CHECK + jitter(60),
            cancel: Arc::new(AtomicBool::new(false)),
            installing_since: None,
            pending_install: None,
        }
    }

    /// 启动时看看上次是不是在安装更新：装成了提示“已更新到 …”，没装成记一笔。
    /// 返回值为真时要打开设置界面（安装前它开着）。
    pub(crate) fn startup(&mut self) -> (Vec<Outcome>, bool) {
        let Some(installing) = self.state.installing.take() else {
            return (Vec::new(), false);
        };
        let _ = self.store.save_update_state(&self.state);
        let current = self.current.to_string();
        if installing.to == current {
            info!(from = %installing.from, to = %current, "已更新");
            clear_downloads(&self.store.updates_dir());
            let phrase = Phrase::new("update_installed")
                .arg("from", &installing.from)
                .arg("to", &current);
            let outcomes = vec![
                Outcome::Log {
                    warning: false,
                    phrase: phrase.clone(),
                    text: format!("已更新到 {current}"),
                    detail: format!("从 {}", installing.from),
                },
                Outcome::Toast(
                    Phrase::new("update_installed").arg("to", &current),
                    format!("已更新到 {current}"),
                ),
            ];
            (outcomes, installing.open_settings)
        } else {
            warn!(to = %installing.to, now = %current, "更新没有装上");
            let outcomes = vec![Outcome::Log {
                warning: true,
                phrase: Phrase::new("update_not_installed")
                    .arg("version", &installing.to)
                    .arg("current", &current),
                text: format!("{} 没有装上", installing.to),
                detail: format!("还在用 {current}"),
            }];
            (outcomes, installing.open_settings)
        }
    }

    /// 每个节拍调用：到时间就自动检查；Windows 的安装包太久没动静就当没装成。
    pub(crate) fn tick(&mut self, auto: bool) -> Vec<Outcome> {
        let now = Instant::now();
        if auto && now >= self.next_check && self.idle() {
            self.check(false);
        }
        if self.status == UpdateStatus::Installing
            && self
                .installing_since
                .is_some_and(|at| at.elapsed() > INSTALL_TIMEOUT)
        {
            self.installing_since = None;
            self.forget_installing();
            self.status = UpdateStatus::Ready;
            return vec![Outcome::Toast(
                Phrase::new("update_failed").arg("message", "安装没有完成"),
                "安装没有完成".into(),
            )];
        }
        Vec::new()
    }

    /// 没有正在检查、下载或安装。
    fn idle(&self) -> bool {
        !matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading | UpdateStatus::Installing
        )
    }

    pub(crate) fn check(&mut self, manual: bool) {
        if !self.idle() {
            return;
        }
        self.manual = manual;
        self.status = UpdateStatus::Checking;
        self.error = None;
        let platform = self.platform.clone();
        let source = self.source.clone();
        let tx = self.tx.clone();
        let current = self.current.to_string();
        tokio::task::spawn_blocking(move || {
            let result = fetch_release(&*platform, &source, &current);
            let _ = tx.send(UpdateEvent::Checked(result));
        });
    }

    pub(crate) fn download(&mut self) {
        if !self.idle() {
            return;
        }
        let (Some(release), Some(platform_name)) = (&self.release, self.platform.platform()) else {
            return;
        };
        let Some(asset) = release.asset(platform_name).cloned() else {
            return;
        };
        let dir = self.store.updates_dir();
        let path = dir.join(safe_file_name(&asset));
        self.status = UpdateStatus::Downloading;
        self.error = None;
        self.done = 0;
        self.total = asset.size;
        self.cancel = Arc::new(AtomicBool::new(false));
        let (platform, tx, cancel) = (self.platform.clone(), self.tx.clone(), self.cancel.clone());
        let current = self.current.to_string();
        tokio::task::spawn_blocking(move || {
            let result = fetch_package(&*platform, &asset, &dir, &path, &current, &tx, &cancel);
            let _ = tx.send(UpdateEvent::Downloaded(result));
        });
    }

    pub(crate) fn cancel(&mut self) {
        if self.status == UpdateStatus::Downloading {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 安装下载好的新版本。`open_settings`：设置界面开着，装好后重新打开。
    pub(crate) fn install(&mut self, open_settings: bool) {
        if self.status != UpdateStatus::Ready {
            return;
        }
        let (Some(package), Some(release)) = (self.package.clone(), &self.release) else {
            return;
        };
        let version = release.version.to_string();
        self.state.installing = Some(Installing {
            from: self.current.to_string(),
            to: version.clone(),
            open_settings,
        });
        let _ = self.store.save_update_state(&self.state);
        self.status = UpdateStatus::Installing;
        self.error = None;
        let (platform, tx) = (self.platform.clone(), self.tx.clone());
        tokio::task::spawn_blocking(move || {
            let result = platform
                .prepare(&package, &version)
                .map_err(|e| e.to_string());
            let _ = tx.send(UpdateEvent::Prepared(result));
        });
    }

    pub(crate) fn dismiss(&mut self, version: String) {
        self.state.dismissed = version;
        let _ = self.store.save_update_state(&self.state);
    }

    pub(crate) fn on_event(&mut self, event: UpdateEvent, auto: bool) -> Vec<Outcome> {
        match event {
            UpdateEvent::Checked(result) => self.on_checked(result, auto),
            UpdateEvent::Progress { done, total } => {
                self.done = done;
                self.total = total;
                Vec::new()
            }
            UpdateEvent::Downloaded(Ok(path)) => {
                info!(path = %path.display(), "新版本已下载并核对");
                self.package = Some(path);
                self.status = UpdateStatus::Ready;
                Vec::new()
            }
            UpdateEvent::Downloaded(Err(None)) => {
                self.status = UpdateStatus::Available;
                Vec::new()
            }
            UpdateEvent::Downloaded(Err(Some((error, message)))) => {
                warn!("下载新版本失败：{message}");
                self.fail(error, message);
                Vec::new()
            }
            UpdateEvent::Prepared(Ok(prepared)) => {
                if prepared.restart_self {
                    self.pending_install = Some(prepared);
                    return vec![Outcome::Restart];
                }
                let (platform, tx) = (self.platform.clone(), self.tx.clone());
                tokio::task::spawn_blocking(move || {
                    let result = platform.launch(&prepared);
                    let _ = tx.send(UpdateEvent::Launched(result));
                });
                Vec::new()
            }
            UpdateEvent::Prepared(Err(message)) => {
                warn!("准备安装失败：{message}");
                self.forget_installing();
                // 可能是更新包有问题，删掉，下次重新下载。
                if let Some(package) = self.package.take() {
                    let _ = std::fs::remove_file(package);
                }
                self.fail(UpdateError::InstallFailed, message.clone());
                vec![Outcome::Log {
                    warning: true,
                    phrase: Phrase::new("update_failed").arg("message", &message),
                    text: "更新没有装上".into(),
                    detail: message,
                }]
            }
            UpdateEvent::Launched(Ok(())) => {
                info!("安装包已经开始运行");
                self.installing_since = Some(Instant::now());
                Vec::new()
            }
            UpdateEvent::Launched(Err(ferry_platform::Error::Cancelled)) => {
                self.forget_installing();
                self.status = UpdateStatus::Ready;
                vec![Outcome::Toast(
                    Phrase::new("update_cancelled"),
                    "没有得到管理员确认，没有安装".into(),
                )]
            }
            UpdateEvent::Launched(Err(e)) => {
                self.forget_installing();
                self.fail(UpdateError::InstallFailed, e.to_string());
                Vec::new()
            }
        }
    }

    fn on_checked(
        &mut self,
        result: Result<Release, (UpdateError, String)>,
        auto: bool,
    ) -> Vec<Outcome> {
        self.checked_at = Some(now_ms());
        let release = match result {
            Ok(release) => release,
            Err((error, message)) => {
                warn!("检查更新失败：{message}");
                if self.manual || error != UpdateError::Unreachable {
                    self.fail(error, message);
                } else {
                    // 自动检查连不上：不打扰，过一会儿再试。
                    self.status = self.resting_status();
                    self.next_check = Instant::now() + RETRY_AFTER;
                }
                return Vec::new();
            }
        };
        self.next_check = Instant::now() + CHECK_EVERY + jitter(3600);
        let verdict = judge(
            &release,
            &self.current,
            now_ms() / 1000,
            self.state.seen_published,
        );
        match verdict {
            Verdict::Expired | Verdict::Rollback => {
                warn!(?verdict, version = %release.version, "更新清单不可信");
                self.fail(
                    UpdateError::Stale,
                    "拿到的更新信息已经过期，无法确认是不是最新版".into(),
                );
                return Vec::new();
            }
            Verdict::UpToDate | Verdict::Newer => {}
        }
        if self
            .state
            .seen_published
            .is_none_or(|seen| release.published > seen)
        {
            self.state.seen_published = Some(release.published);
            let _ = self.store.save_update_state(&self.state);
        }
        if verdict == Verdict::UpToDate {
            self.release = None;
            self.package = None;
            self.status = UpdateStatus::UpToDate;
            return Vec::new();
        }
        let same = self
            .release
            .as_ref()
            .is_some_and(|r| r.version == release.version);
        if !same {
            info!(version = %release.version, "发现新版本");
            self.package = None;
        }
        self.release = Some(release);
        if self.package.is_some() {
            self.status = UpdateStatus::Ready;
        } else {
            self.status = UpdateStatus::Available;
            if auto && self.can_install() {
                self.download();
            }
        }
        Vec::new()
    }

    fn fail(&mut self, error: UpdateError, message: String) {
        self.status = UpdateStatus::Failed;
        self.error = Some((error, message));
    }

    /// 检查失败后回到的状态：有新版本时仍然显示它。
    fn resting_status(&self) -> UpdateStatus {
        match (&self.release, &self.package) {
            (Some(_), Some(_)) => UpdateStatus::Ready,
            (Some(_), None) => UpdateStatus::Available,
            _ if self.checked_at.is_some() => UpdateStatus::UpToDate,
            _ => UpdateStatus::Idle,
        }
    }

    fn forget_installing(&mut self) {
        self.state.installing = None;
        let _ = self.store.save_update_state(&self.state);
    }

    /// 能自己装：平台支持，而且新版本有这个平台的更新包。
    fn can_install(&self) -> bool {
        let has_asset = match (&self.release, self.platform.platform()) {
            (Some(release), Some(name)) => release.asset(name).is_some(),
            (None, Some(_)) => true,
            _ => false,
        };
        has_asset && self.platform.can_install().is_ok()
    }

    /// 安装要弹出管理员确认（Windows）。
    pub(crate) fn needs_elevation(&self) -> bool {
        self.platform
            .platform()
            .is_some_and(|name| name.starts_with("windows"))
    }

    /// 托盘菜单用：下载好、可以安装的版本。
    pub(crate) fn ready_version(&self) -> Option<String> {
        (self.status == UpdateStatus::Ready)
            .then(|| self.release.as_ref().map(|r| r.version.to_string()))
            .flatten()
    }

    /// macOS：交给 [`finish`] 的安装。
    pub(crate) fn take_restart(&mut self) -> Option<(Arc<U>, Prepared)> {
        let prepared = self.pending_install.take()?;
        Some((self.platform.clone(), prepared))
    }

    pub(crate) fn info(&self, needs_local_confirm: bool) -> UpdateInfo {
        let latest = self.release.as_ref().map(|r| UpdateRelease {
            version: r.version.to_string(),
            published: r.published * 1000,
            size: self
                .platform
                .platform()
                .and_then(|name| r.asset(name))
                .map_or(0, |a| a.size),
            notes_zh: r.notes.zh.clone(),
            notes_en: r.notes.en.clone(),
            breaks_peers: r.protocol != ferry_proto::WIRE_FORMAT,
        });
        let (error, message) = match &self.error {
            Some((e, m)) => (Some(*e), m.clone()),
            None => (None, String::new()),
        };
        UpdateInfo {
            status: self.status,
            latest,
            done: self.done,
            total: self.total,
            error,
            message,
            checked_at: self.checked_at,
            can_install: self.can_install(),
            needs_local_confirm: needs_local_confirm && self.needs_elevation(),
            dismissed: self.state.dismissed.clone(),
        }
    }
}

/// 取清单和签名，校验后解析。
fn fetch_release<U: Updater + ?Sized>(
    platform: &U,
    source: &UpdateSource,
    current: &str,
) -> Result<Release, (UpdateError, String)> {
    let keys: Vec<&str> = source.keys.iter().map(String::as_str).collect();
    if keys.is_empty() {
        return Err((
            UpdateError::NoKeys,
            "这份程序里没有发布公钥，无法校验更新".into(),
        ));
    }
    let https = client(platform, current)?;
    let manifest = https
        .fetch(&source.manifest_url, MANIFEST_LIMIT)
        .map_err(unreachable)?;
    let signature = https
        .fetch(&format!("{}.minisig", source.manifest_url), SIGNATURE_LIMIT)
        .map_err(unreachable)?;
    let signature = String::from_utf8_lossy(&signature);
    Release::verify(&manifest, &signature, &keys)
        .map_err(|e| (UpdateError::VerifyFailed, e.to_string()))
}

/// 下载更新包到 `path`，核对大小和 SHA-256。对不上就删掉。
fn fetch_package<U: Updater + ?Sized>(
    platform: &U,
    asset: &Asset,
    dir: &Path,
    path: &Path,
    current: &str,
    tx: &mpsc::UnboundedSender<UpdateEvent>,
    cancel: &AtomicBool,
) -> Result<PathBuf, Option<(UpdateError, String)>> {
    std::fs::create_dir_all(dir).map_err(|e| Some((UpdateError::InstallFailed, e.to_string())))?;
    // 以前下的别的版本不要了。
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let other = entry.path();
            if other != path
                && other.file_name().is_some_and(|n| n != "state.toml")
                && other.is_file()
            {
                let _ = std::fs::remove_file(other);
            }
        }
    }
    let https = client(platform, current).map_err(Some)?;
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut progress = |done: u64, total: u64| {
        if done == total || last.elapsed() >= Duration::from_millis(250) {
            last = Instant::now();
            let _ = tx.send(UpdateEvent::Progress { done, total });
        }
    };
    let downloaded = https
        .download(&asset.url, path, asset.size, &mut progress, cancel)
        .map_err(|e| match e {
            ferry_net::Error::Cancelled => None,
            ferry_net::Error::TooLarge => {
                let _ = std::fs::remove_file(path);
                Some((UpdateError::VerifyFailed, "下载的文件比预期的大".into()))
            }
            e => Some(unreachable(e)),
        })?;
    if !asset.matches(downloaded.size, &downloaded.sha256) {
        let _ = std::fs::remove_file(path);
        return Err(Some((
            UpdateError::VerifyFailed,
            "下载的文件和更新清单对不上，已删除".into(),
        )));
    }
    Ok(path.to_owned())
}

fn client<U: Updater + ?Sized>(
    platform: &U,
    current: &str,
) -> Result<Https, (UpdateError, String)> {
    let proxy = platform.proxy();
    let agent = format!(
        "ScreenFerry/{current} ({})",
        platform.platform().unwrap_or("unknown")
    );
    Https::new(proxy.as_deref(), &agent).map_err(unreachable)
}

fn unreachable(e: ferry_net::Error) -> (UpdateError, String) {
    (UpdateError::Unreachable, e.to_string())
}

/// 更新包存成什么名字：下载地址里的文件名，只留安全的字符。
fn safe_file_name(asset: &Asset) -> String {
    let name: String = asset
        .file_name()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    if name.is_empty() || name.starts_with('.') {
        "update.bin".to_owned()
    } else {
        name
    }
}

/// 删掉下载的更新包，只留状态文件。
fn clear_downloads(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().is_some_and(|n| n != "state.toml") {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

/// 本进程退出之前（macOS）：启动换好的新版本，等它起来；起不来就换回旧版本再启动。
pub(crate) async fn finish<U: Updater + 'static>(
    platform: Arc<U>,
    prepared: Prepared,
    endpoint: PathBuf,
) {
    let launched = {
        let (platform, prepared) = (platform.clone(), prepared.clone());
        tokio::task::spawn_blocking(move || platform.launch(&prepared)).await
    };
    let started = match launched {
        Ok(Ok(())) => wait_for_new_version(&endpoint).await,
        Ok(Err(e)) => {
            warn!("启动新版本失败：{e}");
            false
        }
        Err(_) => false,
    };
    tokio::task::spawn_blocking(move || {
        if started {
            info!("新版本已经起来了");
            platform.confirm(&prepared);
            return;
        }
        warn!("新版本没有起来，换回旧版本");
        match platform.rollback(&prepared) {
            Ok(()) => {
                if let Err(e) = platform.launch(&prepared) {
                    warn!("重新启动旧版本失败：{e}");
                }
            }
            Err(e) => warn!("换回旧版本失败：{e}"),
        }
    })
    .await
    .ok();
}

/// 等另一个进程写下 `ipc.json` 并能连上。
async fn wait_for_new_version(path: &Path) -> bool {
    let own = std::process::id();
    let deadline = Instant::now() + NEW_VERSION_WAIT;
    while Instant::now() < deadline {
        if let Ok(Some(endpoint)) = Endpoint::load(path)
            && endpoint.pid != own
            && ferry_ipc::connect(&endpoint).await.is_ok()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// 不超过 `max_secs` 秒的随机偏移：错开大家检查的时间。
fn jitter(max_secs: u64) -> Duration {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::from(d.subsec_nanos()));
    Duration::from_secs(nanos % (max_secs + 1))
}
