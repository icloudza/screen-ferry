//! 应用主流程：按平台选择实现，启动引擎。

use std::convert::Infallible;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use ferry_engine::{ClientConfig, EdgeFeed, Events, Files, Live, ServerAddr, ServerConfig};
use ferry_store::Store;
use ferry_trust::Fingerprint;
use ferry_types::{Config, DeviceSettings, ModifierPreset, uses_command_key};
use tracing::{error, info};

use crate::cli::{Mode, Options, Parsed};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::pairing::TerminalConfirm;

type AppResult = Result<Infallible, Box<dyn std::error::Error>>;

/// `serve`、`connect` 在自己开的控制台窗口里运行（Windows，见 [`init_output`]）。
static OWN_CONSOLE: AtomicBool = AtomicBool::new(false);

/// 准备输出，然后初始化日志。
///
/// Windows 上本程序没有控制台：双击、登录时启动不会闪出黑色窗口。命令行用法先接一个：
/// `serve`、`connect` 要确认配对码、按 Ctrl+C 退出，新开一个窗口；其他命令接回启动它的终端。
pub(crate) fn init_output(parsed: &Result<Parsed, String>) {
    #[cfg(target_os = "windows")]
    {
        let own = match parsed {
            Ok(Parsed::Run(Options {
                mode: Mode::Resident { .. },
                ..
            })) => None,
            Ok(Parsed::Run(Options {
                mode: Mode::Serve { .. } | Mode::Connect { .. },
                ..
            })) => Some(true),
            _ => Some(false),
        };
        if let Some(own) = own {
            let attached = ferry_platform_windows::attach_console(own);
            OWN_CONSOLE.store(own && attached, Ordering::Relaxed);
        }
    }
    if let Ok(Parsed::Run(options)) = parsed {
        crate::logging::init(options);
    }
}

pub(crate) fn run(options: Options) -> ExitCode {
    let store = match &options.data_dir {
        Some(dir) => Store::open(dir),
        None => Store::open_default(),
    };
    let store = match store {
        Ok(store) => store,
        Err(e) => {
            error!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match options.mode {
        Mode::Resident { autostart } => return report(resident(store, autostart)),
        Mode::Peers => return report(list_peers(&store)),
        Mode::InputKey => return report(input_key(&store, &options.name)),
        Mode::Forget { target } => return report(forget(&store, &target)),
        Mode::Send { paths } => return report(send_files(&store, paths)),
        Mode::Quit => return report(quit(&store)),
        Mode::PreviewCapsule { dir } => return report(crate::preview::write(&dir)),
        Mode::Serve { .. } | Mode::Connect { .. } => {}
    }

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            error!("无法启动运行时：{e}");
            return ExitCode::FAILURE;
        }
    };
    let files = Files::new(&store, Events::none());
    let result = runtime.block_on(async move {
        let work = async {
            match options.mode {
                Mode::Serve {
                    listen,
                    side,
                    swap_cmd_ctrl,
                    announce,
                } => {
                    let fallback = DeviceSettings {
                        modifiers: if swap_cmd_ctrl {
                            ModifierPreset::Habit
                        } else {
                            ModifierPreset::Plain
                        },
                        ..DeviceSettings::default()
                    };
                    let config = ServerConfig {
                        live: Live {
                            side,
                            layout: Vec::new(),
                            devices: Vec::new(),
                            fallback,
                            mouse_dpi: Config::default().mouse_dpi,
                        }
                        .fixed(),
                        name: options.name,
                        clipboard: options.clipboard,

                        clipboard_limit: Default::default(),
                        command_key: uses_command_key(),
                        store,
                        announce,
                        events: Events::none(),
                        files,
                        edge: EdgeFeed::none(),
                    };
                    serve(listen, config).await
                }
                Mode::Connect { server } => {
                    let config = ClientConfig {
                        server: server.map_or(ServerAddr::Discover, ServerAddr::Fixed),
                        name: options.name,
                        clipboard: options.clipboard,

                        clipboard_limit: Default::default(),
                        command_key: uses_command_key(),
                        store,
                        events: Events::none(),
                        files,
                        edge: EdgeFeed::none(),
                        devices: Vec::new(),
                        commands: None,
                        invite_port: None,
                    };
                    connect(config).await
                }
                Mode::Resident { .. }
                | Mode::Peers
                | Mode::InputKey
                | Mode::Forget { .. }
                | Mode::Send { .. }
                | Mode::Quit
                | Mode::PreviewCapsule { .. } => {
                    unreachable!("已在上面处理")
                }
            }
        };
        // 收到 Ctrl+C 时让引擎正常析构：归还鼠标控制、松开按键，而不是被系统直接杀掉。
        tokio::select! {
            result = work => result.map(|never| match never {}),
            _ = tokio::signal::ctrl_c() => {
                info!("收到 Ctrl+C，正在退出");
                Ok(())
            }
        }
    });
    let code = report(result);
    // 自己开的控制台窗口随进程一起关闭：先停一下，让人看清最后的输出。
    if OWN_CONSOLE.load(Ordering::Relaxed) {
        eprintln!("\n按回车键关闭这个窗口");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    code
}

/// 常驻运行。已经有一份在运行时，请它打开设置界面，然后退出。
///
/// 主线程运行系统事件循环和托盘图标；引擎与本机通信在另一个线程的 tokio 运行时里。
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn resident(store: Store, autostart: bool) -> Result<(), Box<dyn std::error::Error>> {
    use ferry_engine::resident::{self, Command};
    use ferry_platform::EventLoop;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    if runtime.block_on(resident::hand_over_to_running(&store)) {
        info!("ScreenFerry 已在运行，已请它打开设置界面");
        return Ok(());
    }

    let mut event_loop = crate::host::NativeEventLoop::new()?;
    let dispatcher = event_loop.dispatcher();
    let (control, handle) = resident::control();
    crate::tray::install(handle.clone())?;
    event_loop.on_reopen(Box::new({
        let handle = handle.clone();
        move || handle.send(Command::OpenSettings)
    }));
    // 用户手动启动时打开设置界面；登录时由系统启动则安静地待在菜单栏里。
    if !autostart {
        handle.send(Command::OpenSettings);
    }

    std::thread::Builder::new()
        .name("ferry-runtime".into())
        .spawn(move || {
            let result = runtime.block_on(async move {
                tokio::spawn(crate::tray::follow(handle.summary(), dispatcher));
                // Ctrl+C 走正常退出：停下引擎（归还鼠标、松开按键），删掉 ipc.json。
                tokio::spawn({
                    let handle = handle.clone();
                    async move {
                        if tokio::signal::ctrl_c().await.is_ok() {
                            info!("收到 Ctrl+C，正在退出");
                            handle.send(Command::Quit);
                        }
                    }
                });
                let result = resident::run(store, crate::host::NativeHost::new(), control).await;
                drop(handle);
                result
            });
            // 退出菜单或 Ctrl+C：引擎已经停下，主线程的事件循环随进程一起结束。
            match result {
                Ok(()) => std::process::exit(0),
                Err(e) => {
                    error!("{e}");
                    std::process::exit(1);
                }
            }
        })?;
    event_loop.run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn resident(_store: Store, _autostart: bool) -> Result<(), Box<dyn std::error::Error>> {
    Err("常驻运行目前只支持 macOS 和 Windows".into())
}

fn report(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn list_peers(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    let peers = store.peers()?;
    println!("数据目录：{}", store.root().display());
    if peers.is_empty() {
        println!("还没有配对过任何设备。");
    }
    for peer in peers.iter() {
        println!("  {}  {}", peer.fingerprint, peer.name);
    }
    Ok(())
}

/// 请正在运行的后台发送文件。路径先转成绝对路径，后台的工作目录与这里不同。
fn send_files(
    store: &Store,
    paths: Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut absolute = Vec::with_capacity(paths.len());
    for path in paths {
        let path =
            std::fs::canonicalize(&path).map_err(|e| format!("找不到 {}：{e}", path.display()))?;
        absolute.push(path.to_string_lossy().into_owned());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let reply = runtime.block_on(ferry_engine::resident::send_via_running(store, absolute))?;
    println!("{reply}");
    Ok(())
}

/// 请正在运行的后台退出，等它退出后返回。
fn quit(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    if runtime.block_on(ferry_engine::resident::quit_running(store))? {
        println!("ScreenFerry 已退出。");
    } else {
        println!("ScreenFerry 没有在运行。");
    }
    Ok(())
}

/// 按设备名，或者指纹（完整十六进制，或 `peers` 显示的短形式及其前缀）删除一台设备。
fn forget(store: &Store, target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut peers = store.peers()?;
    let wanted = target.to_uppercase();
    let matches: Vec<Fingerprint> = peers
        .iter()
        .filter(|p| {
            p.name == target
                || p.fingerprint.to_hex().to_uppercase() == wanted
                || (wanted.len() >= 4 && p.fingerprint.short().starts_with(&wanted))
        })
        .map(|p| p.fingerprint)
        .collect();
    match matches.as_slice() {
        [] => Err(format!("没有找到 {target}，可先用 peers 查看").into()),
        [fingerprint] => {
            peers.remove(fingerprint);
            store.save_peers(&peers)?;
            println!("已删除 {fingerprint}，下次连接需要重新配对。");
            Ok(())
        }
        _ => Err(format!("{target} 对应多台设备，请改用更长的指纹").into()),
    }
}

fn input_key(store: &Store, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let identity = store.identity(name)?;
    let signer = ferry_trust::InputSigner::new(&identity)?;
    println!("{}", signer.authorization().to_code());
    Ok(())
}

#[cfg(target_os = "macos")]
async fn serve(listen: SocketAddr, config: ServerConfig) -> AppResult {
    use ferry_platform::Permissions;
    use ferry_platform_macos::{MacCapture, MacClipboard, MacDisplays, MacPermissions};

    MacPermissions.check_input()?;
    let listener = tokio::net::TcpListener::bind(listen).await?;
    let (capture, clipboard) = (MacCapture::new(), MacClipboard::new());
    let engine = ferry_engine::serve(
        listener,
        config,
        capture,
        MacDisplays,
        clipboard,
        TerminalConfirm,
    );
    Ok(engine.await?)
}

#[cfg(target_os = "windows")]
async fn serve(listen: SocketAddr, config: ServerConfig) -> AppResult {
    use ferry_platform_windows::{WinCapture, WinClipboard, WinDisplays};

    let listener = tokio::net::TcpListener::bind(listen).await?;
    let (capture, displays) = (WinCapture::new(), WinDisplays::new());
    let engine = ferry_engine::serve(
        listener,
        config,
        capture,
        displays,
        WinClipboard::new(),
        TerminalConfirm,
    );
    Ok(engine.await?)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn serve(_listen: SocketAddr, _config: ServerConfig) -> AppResult {
    Err("目前只能在 macOS 和 Windows 上运行 serve".into())
}

#[cfg(target_os = "macos")]
async fn connect(config: ClientConfig) -> AppResult {
    use ferry_platform::Permissions;
    use ferry_platform_macos::{MacClipboard, MacDisplays, MacInject, MacPermissions};

    MacPermissions.check_input()?;
    let engine = ferry_engine::connect(
        config,
        MacInject::new()?,
        MacDisplays,
        MacClipboard::new(),
        TerminalConfirm,
    );
    Ok(engine.await?)
}

#[cfg(target_os = "windows")]
async fn connect(config: ClientConfig) -> AppResult {
    use ferry_platform::Permissions;
    use ferry_platform_windows::{WinClipboard, WinDisplays, WinInject, WinPermissions};

    WinPermissions.check_input()?;
    let (inject, displays) = (WinInject::new(), WinDisplays::new());
    let engine = ferry_engine::connect(
        config,
        inject,
        displays,
        WinClipboard::new(),
        TerminalConfirm,
    );
    Ok(engine.await?)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn connect(_config: ClientConfig) -> AppResult {
    Err("目前只能在 macOS 和 Windows 上运行 connect".into())
}
