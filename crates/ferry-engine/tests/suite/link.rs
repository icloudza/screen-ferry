//! 握手参数：平台与快捷键、边缘投递能力各自独立。

use std::time::Duration;

use ferry_core::Side;
use ferry_engine::{
    ClientConfig, ClipLimit, EdgeFeed, Event, Events, Files, Live, ServerAddr, ServerConfig,
    connect, serve,
};
use ferry_net::{Framed, Tls};
use ferry_platform_mock::{MockCapture, MockClipboard, MockDisplays, MockInject};
use ferry_proto::{Capabilities, Channel, Hello, Message};
use ferry_trust::{Identity, Peer};
use ferry_types::DeviceSettings;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

use crate::support::{AutoConfirm, Device, display};

fn trust(device: &Device, name: &str) -> Identity {
    let identity = device.store.identity(name).unwrap();
    let mut peers = device.store.peers().unwrap();
    peers.add(Peer {
        name: name.to_owned(),
        fingerprint: identity.fingerprint(),
        addr: None,
    });
    device.store.save_peers(&peers).unwrap();
    identity
}

fn live() -> tokio::sync::watch::Receiver<Live> {
    Live {
        side: Side::Right,
        layout: Vec::new(),
        devices: Vec::new(),
        fallback: DeviceSettings::default(),
        mouse_dpi: 1600,
    }
    .fixed()
}

async fn connected(events: &mut UnboundedReceiver<Event>) -> Event {
    while let Some(event) = events.recv().await {
        if matches!(event, Event::Connected { .. }) {
            return event;
        }
    }
    panic!("事件通道关闭，没有收到连接事件")
}

async fn connected_platform(events: &mut UnboundedReceiver<Event>) -> (Option<String>, String) {
    let Event::Connected {
        platform: connected,
        ..
    } = connected(events).await
    else {
        unreachable!();
    };
    while let Some(event) = events.recv().await {
        if let Event::PeerPlatform { platform, .. } = event {
            return (connected, platform);
        }
    }
    panic!("事件通道关闭，没有收到对端系统信息")
}

async fn assert_no_edge_drop<S>(framed: &mut Framed<S>)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        match timeout(Duration::from_millis(150), framed.recv()).await {
            Err(_) => return,
            Ok(Ok(Some(Message::Ping { nonce }))) => {
                framed.send(&Message::Pong { nonce }).await.unwrap();
            }
            Ok(Ok(Some(message @ (Message::PeerSide { .. } | Message::DropHint(_))))) => {
                panic!("未协商边缘投递却收到 {message:?}");
            }
            Ok(Ok(Some(_))) | Ok(Ok(None)) => return,
            Ok(Err(error)) => panic!("接收消息失败：{error}"),
        }
    }
}

async fn tls_connect(tls: &Tls, addr: std::net::SocketAddr) -> Framed<ferry_net::ClientStream> {
    let (framed, _) = timeout(Duration::from_secs(5), tls.connect(addr, Channel::Input))
        .await
        .expect("TLS 连接超时")
        .unwrap();
    framed
}

async fn tls_accept(tls: &Tls, stream: TcpStream) -> Framed<ferry_net::ServerStream> {
    let (framed, _) = timeout(Duration::from_secs(5), tls.accept(stream))
        .await
        .expect("TLS 接受超时")
        .unwrap();
    framed
}

#[tokio::test]
async fn server_reports_explicit_linux_without_edge_drop() {
    let receiver = Device::new();
    let identity = trust(&receiver, "controller");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (events, mut event_rx) = Events::channel();
    let config = ServerConfig {
        live: live(),
        name: "receiver".into(),
        clipboard: false,
        clipboard_limit: ClipLimit::default(),
        platform: "linux",
        command_key: false,
        edge_drop: false,
        store: receiver.store.clone(),
        announce: false,
        events,
        files: Files::new(&receiver.store, Events::none()),
        edge: EdgeFeed::none(),
    };
    let server = tokio::spawn(serve(
        listener,
        config,
        MockCapture::new(),
        MockDisplays::new(vec![display(1000, 800, 100)]),
        MockClipboard::new(),
        AutoConfirm::answering(true),
    ));

    let tls = Tls::new(&identity).unwrap();
    let mut framed = tls_connect(&tls, addr).await;
    let welcome = timeout(
        Duration::from_secs(5),
        ferry_net::initiate(
            &mut framed,
            Hello::new(
                Channel::Input,
                Capabilities::EDGE_DROP
                    | Capabilities::COMMAND_KEY
                    | Capabilities::APP_INFO
                    | Capabilities::PLATFORM_INFO,
                0,
                "controller",
            ),
        ),
    )
    .await
    .expect("握手超时")
    .unwrap();
    assert!(!welcome.capabilities.contains(Capabilities::EDGE_DROP));
    assert!(!welcome.capabilities.contains(Capabilities::COMMAND_KEY));
    assert!(welcome.capabilities.contains(Capabilities::PLATFORM_INFO));
    assert!(matches!(
        timeout(Duration::from_secs(5), framed.recv())
            .await
            .expect("等待显示器信息超时")
            .unwrap(),
        Some(Message::Displays(_))
    ));
    timeout(
        Duration::from_secs(5),
        framed.send(&Message::Displays(vec![display(1920, 1080, 150)])),
    )
    .await
    .expect("发送显示器信息超时")
    .unwrap();
    assert!(matches!(
        timeout(Duration::from_secs(5), framed.recv())
            .await
            .expect("等待版本信息超时")
            .unwrap(),
        Some(Message::AppInfo { .. })
    ));
    assert!(matches!(
        timeout(Duration::from_secs(5), framed.recv())
            .await
            .expect("等待系统信息超时")
            .unwrap(),
        Some(Message::PlatformInfo { platform }) if platform == "linux"
    ));
    timeout(
        Duration::from_secs(5),
        framed.send(&Message::PlatformInfo {
            platform: "macos".into(),
        }),
    )
    .await
    .expect("发送系统信息超时")
    .unwrap();
    assert_no_edge_drop(&mut framed).await;
    assert_eq!(
        timeout(Duration::from_secs(5), connected_platform(&mut event_rx))
            .await
            .unwrap(),
        (None, "macos".to_owned())
    );

    drop(framed);
    server.abort();
}

#[tokio::test]
async fn client_reports_explicit_linux_without_edge_drop() {
    let controller = Device::new();
    let receiver = Device::new();
    let controller_identity = trust(&receiver, "controller");
    trust(&controller, "receiver");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let tls = Tls::new(&controller_identity).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = tls_accept(&tls, stream).await;
        let (hello, _) =
            ferry_net::accept(&mut framed, Capabilities::all(), "controller", |_| Ok(1))
                .await
                .unwrap();
        assert_eq!(
            hello.capabilities & (Capabilities::EDGE_DROP | Capabilities::COMMAND_KEY),
            Capabilities::empty()
        );
        assert!(hello.capabilities.contains(Capabilities::PLATFORM_INFO));
        framed
            .send(&Message::Displays(vec![display(1000, 800, 100)]))
            .await
            .unwrap();
        assert!(matches!(
            framed.recv().await.unwrap(),
            Some(Message::Displays(_))
        ));
        assert!(matches!(
            framed.recv().await.unwrap(),
            Some(Message::AppInfo { .. })
        ));
        assert!(matches!(
            framed.recv().await.unwrap(),
            Some(Message::PlatformInfo { platform }) if platform == "linux"
        ));
        framed
            .send(&Message::PlatformInfo {
                platform: "windows".into(),
            })
            .await
            .unwrap();
        assert_no_edge_drop(&mut framed).await;
    });

    let (events, mut event_rx) = Events::channel();
    let config = ClientConfig {
        server: ServerAddr::Fixed(addr),
        name: "receiver".into(),
        clipboard: false,
        clipboard_limit: ClipLimit::default(),
        platform: "linux",
        command_key: false,
        edge_drop: false,
        store: receiver.store.clone(),
        events,
        files: Files::new(&receiver.store, Events::none()),
        edge: EdgeFeed::none(),
        devices: Vec::new(),
        commands: None,
        invite_port: None,
    };
    let client = tokio::spawn(connect(
        config,
        MockInject::new(),
        MockDisplays::new(vec![display(1920, 1080, 150)]),
        MockClipboard::new(),
        AutoConfirm::answering(true),
    ));

    timeout(Duration::from_secs(5), server)
        .await
        .expect("服务端握手超时")
        .unwrap();
    assert_eq!(
        timeout(Duration::from_secs(5), connected_platform(&mut event_rx))
            .await
            .unwrap(),
        (None, "windows".to_owned())
    );
    client.abort();
}

#[tokio::test]
async fn old_peers_connect_without_platform_info() {
    let receiver = Device::new();
    let identity = trust(&receiver, "controller");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let tls = Tls::new(&identity).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = tls_accept(&tls, stream).await;
        let (hello, welcome) = ferry_net::accept(
            &mut framed,
            Capabilities::all() - Capabilities::PLATFORM_INFO,
            "controller",
            |_| Ok(1),
        )
        .await
        .unwrap();
        assert!(!welcome.capabilities.contains(Capabilities::PLATFORM_INFO));
        assert_eq!(
            hello.capabilities & (Capabilities::EDGE_DROP | Capabilities::COMMAND_KEY),
            Capabilities::empty()
        );
        framed
            .send(&Message::Displays(vec![display(1000, 800, 100)]))
            .await
            .unwrap();
        assert!(matches!(
            framed.recv().await.unwrap(),
            Some(Message::Displays(_))
        ));
        assert!(matches!(
            framed.recv().await.unwrap(),
            Some(Message::AppInfo { .. })
        ));
        loop {
            match timeout(Duration::from_millis(150), framed.recv()).await {
                Err(_) => break,
                Ok(Ok(Some(Message::Ping { nonce }))) => {
                    framed.send(&Message::Pong { nonce }).await.unwrap();
                }
                Ok(Ok(Some(Message::PlatformInfo { .. }))) => {
                    panic!("旧版对端没有协商系统信息，不应收到 PlatformInfo");
                }
                Ok(Ok(Some(_))) | Ok(Ok(None)) => break,
                Ok(Err(error)) => panic!("接收消息失败：{error}"),
            }
        }
    });

    let (events, mut event_rx) = Events::channel();
    let config = ClientConfig {
        server: ServerAddr::Fixed(addr),
        name: "receiver".into(),
        clipboard: false,
        clipboard_limit: ClipLimit::default(),
        platform: "linux",
        command_key: false,
        edge_drop: false,
        store: receiver.store.clone(),
        events,
        files: Files::new(&receiver.store, Events::none()),
        edge: EdgeFeed::none(),
        devices: Vec::new(),
        commands: None,
        invite_port: None,
    };
    let client = tokio::spawn(connect(
        config,
        MockInject::new(),
        MockDisplays::new(vec![display(1920, 1080, 150)]),
        MockClipboard::new(),
        AutoConfirm::answering(true),
    ));

    timeout(Duration::from_secs(5), server)
        .await
        .expect("旧版对端握手超时")
        .unwrap();
    let event = connected(&mut event_rx).await;
    assert!(matches!(event, Event::Connected { platform: None, .. }));
    client.abort();
}
