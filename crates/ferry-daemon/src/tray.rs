//! 托盘图标：显示是否连着另一台电脑，菜单里可以暂停或继续共享、打开设置、退出；
//! 新版本下载好后多一项“重启并更新”。
//! 图标只有两种：已连接（光球正穿过屏幕边框）和未连接（光球停在屏幕里、变淡）；
//! 暂停、等待连接、寻找服务端、出了问题都算未连接，具体情况看菜单第一行。
//! macOS 按菜单栏的习惯，点图标弹出菜单；Windows 和 Linux 左键直接打开设置界面，右键弹出菜单。
//!
//! 托盘对象只能在主线程上创建和修改，所以放在主线程的线程局部变量里；
//! 状态变化时由引擎所在的线程经 [`Dispatcher`] 把更新交给主线程。

use std::cell::RefCell;

use ferry_engine::resident::{Command, Handle, Summary};
use ferry_platform::Dispatcher;
use tokio::sync::watch;
use tracing::warn;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon};

const TOGGLE: &str = "toggle";
const OPEN: &str = "open";
const UPDATE: &str = "update";
const QUIT: &str = "quit";

/// 菜单栏用模板图：只看透明度，系统按明暗模式自动着色。
#[cfg(target_os = "macos")]
mod art {
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    pub(super) const CONNECTED: &[u8] =
        include_bytes!("../../../assets/icons/tray-connected@2x.png");
    pub(super) const DISCONNECTED: &[u8] =
        include_bytes!("../../../assets/icons/tray-disconnected@2x.png");

    pub(super) fn builder(icon: Icon) -> TrayIconBuilder {
        TrayIconBuilder::new().with_icon_templated(icon)
    }

    pub(super) fn set(tray: &TrayIcon, icon: Icon) -> tray_icon::Result<()> {
        tray.set_icon_templated(Some(icon))
    }
}

/// 任务栏可能是深色也可能是浅色，用两种底色上都看得清的彩色图标；未连接时是灰色。
#[cfg(target_os = "windows")]
mod art {
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    pub(super) const CONNECTED: &[u8] =
        include_bytes!("../../../assets/icons/tray-win-connected.png");
    pub(super) const DISCONNECTED: &[u8] =
        include_bytes!("../../../assets/icons/tray-win-disconnected.png");

    pub(super) fn builder(icon: Icon) -> TrayIconBuilder {
        // 左键留给“打开设置界面”，菜单只在右键时弹出。
        TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu_on_left_click(false)
    }

    pub(super) fn set(tray: &TrayIcon, icon: Icon) -> tray_icon::Result<()> {
        tray.set_icon(Some(icon))
    }
}

/// Linux 桌面环境通过 KSNI 显示彩色托盘图标。
#[cfg(target_os = "linux")]
mod art {
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    pub(super) const CONNECTED: &[u8] =
        include_bytes!("../../../assets/icons/tray-win-connected.png");
    pub(super) const DISCONNECTED: &[u8] =
        include_bytes!("../../../assets/icons/tray-win-disconnected.png");

    pub(super) fn builder(icon: Icon) -> TrayIconBuilder {
        TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu_on_left_click(false)
    }

    pub(super) fn set(tray: &TrayIcon, icon: Icon) -> tray_icon::Result<()> {
        tray.set_icon(Some(icon))
    }
}

struct Tray {
    icon: TrayIcon,
    menu: Menu,
    status: MenuItem,
    /// “重启并更新”和它下面的分隔线，有新版本时才插进菜单。
    update: MenuItem,
    update_separator: PredefinedMenuItem,
    update_shown: bool,
    toggle: MenuItem,
    open: MenuItem,
    quit: MenuItem,
    connected: Icon,
    disconnected: Icon,
    shown: Option<Summary>,
}

thread_local! {
    static TRAY: RefCell<Option<Tray>> = const { RefCell::new(None) };
}

/// 在主线程上创建托盘图标，菜单命令转给 `handle`。
pub(crate) fn install(handle: Handle) -> Result<(), Box<dyn std::error::Error>> {
    let status = MenuItem::new("正在启动…", false, None);
    let toggle = MenuItem::with_id(TOGGLE, "暂停共享", true, None);
    let open = MenuItem::with_id(OPEN, "打开设置", true, None);
    let quit = MenuItem::with_id(QUIT, "退出 ScreenFerry", true, None);
    let update = MenuItem::with_id(UPDATE, "", true, None);
    let update_separator = PredefinedMenuItem::separator();
    let menu = Menu::new();
    menu.append_items(&[
        &status,
        &PredefinedMenuItem::separator(),
        &toggle,
        &open,
        &PredefinedMenuItem::separator(),
        &quit,
    ])?;

    let connected = decode(art::CONNECTED)?;
    let disconnected = decode(art::DISCONNECTED)?;
    let icon = art::builder(disconnected.clone())
        .with_tooltip("ScreenFerry")
        .with_menu(Box::new(menu.clone()))
        .build()?;

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

        let handle = handle.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                handle.send(Command::OpenSettings);
            }
        }));
    }

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
        TOGGLE => {
            let sharing = handle.summary().borrow().sharing;
            handle.send(Command::SetSharing(!sharing));
        }
        OPEN => handle.send(Command::OpenSettings),
        UPDATE => handle.send(Command::InstallUpdate),
        QUIT => handle.send(Command::Quit),
        _ => {}
    }));

    TRAY.with(|tray| {
        *tray.borrow_mut() = Some(Tray {
            icon,
            menu,
            status,
            update,
            update_separator,
            update_shown: false,
            toggle,
            open,
            quit,
            connected,
            disconnected,
            shown: None,
        });
    });
    Ok(())
}

/// 状态一变就请主线程更新托盘。在引擎所在的 tokio 运行时里运行。
pub(crate) async fn follow(mut summary: watch::Receiver<Summary>, dispatcher: Dispatcher) {
    loop {
        let current = summary.borrow_and_update().clone();
        dispatcher.run_on_main(move || update(&current));
        if summary.changed().await.is_err() {
            return;
        }
    }
}

/// 在主线程上调用。
fn update(summary: &Summary) {
    TRAY.with(|tray| {
        let mut tray = tray.borrow_mut();
        let Some(tray) = tray.as_mut() else { return };
        if tray.shown.as_ref() == Some(summary) {
            return;
        }
        let lang = summary.lang;
        let status = describe(summary);
        tray.status.set_text(&status);
        tray.toggle.set_text(if summary.sharing {
            lang.pick("暂停共享", "Pause Sharing")
        } else {
            lang.pick("继续共享", "Resume Sharing")
        });
        tray.open.set_text(lang.pick("打开设置", "Open Settings"));
        tray.quit
            .set_text(lang.pick("退出 ScreenFerry", "Quit ScreenFerry"));
        show_update(tray, summary);
        let linked = summary.sharing && summary.problem.is_none() && summary.connected_to.is_some();
        let icon = if linked {
            &tray.connected
        } else {
            &tray.disconnected
        };
        let set = art::set(&tray.icon, icon.clone());
        if let Err(e) = set {
            warn!("更新托盘图标失败：{e}");
        }
        let _ = tray
            .icon
            .set_tooltip(Some(format!("ScreenFerry · {status}")));
        tray.shown = Some(summary.clone());
    });
}

/// 有下载好的新版本时，在“退出”上面插一项“重启并更新到 …”，装好后去掉。
fn show_update(tray: &mut Tray, summary: &Summary) {
    match (&summary.update, tray.update_shown) {
        (Some(version), shown) => {
            tray.update.set_text(summary.lang.pick(
                format!("重启并更新到 {version}"),
                format!("Restart to Update to {version}"),
            ));
            if !shown {
                // 菜单：状态、分隔线、暂停、打开设置、分隔线，然后是这两项，最后是退出。
                let inserted = tray
                    .menu
                    .insert_items(&[&tray.update, &tray.update_separator], 5);
                match inserted {
                    Ok(()) => tray.update_shown = true,
                    Err(e) => warn!("托盘菜单加不上“重启并更新”：{e}"),
                }
            }
        }
        (None, true) => {
            let _ = tray.menu.remove(&tray.update);
            let _ = tray.menu.remove(&tray.update_separator);
            tray.update_shown = false;
        }
        (None, false) => {}
    }
}

/// 菜单第一行的状态文字。
fn describe(summary: &Summary) -> String {
    if let Some(problem) = &summary.problem {
        let short: String = problem.chars().take(28).collect();
        let ellipsis = if problem.chars().count() > 28 {
            "…"
        } else {
            ""
        };
        return format!("⚠︎ {short}{ellipsis}");
    }
    let lang = summary.lang;
    match (&summary.connected_to, summary.sharing) {
        (_, false) => lang.pick("已暂停共享", "Sharing paused").to_owned(),
        (Some(peer), true) => lang.pick(format!("已连接 {peer}"), format!("Connected to {peer}")),
        (None, true) => lang
            .pick("共享中 · 等待连接", "Sharing · Waiting for a connection")
            .to_owned(),
    }
}

fn decode(png_bytes: &[u8]) -> Result<Icon, Box<dyn std::error::Error>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size().ok_or("图标太大")?];
    let info = reader.next_frame(&mut rgba)?;
    rgba.truncate(info.buffer_size());
    Ok(Icon::from_rgba(rgba, info.width, info.height)?)
}
