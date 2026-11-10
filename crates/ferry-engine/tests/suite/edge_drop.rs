use std::fs;
use std::sync::Arc;
use std::time::Duration;

use ferry_core::capsule::hit;
use ferry_engine::{EdgeDrop, EdgeFeed, Event, Events, Files};
use ferry_platform::{CaptureEvent, OverlayEvent};
use ferry_platform_mock::MockOverlay;
use ferry_types::{Lang, MouseButton, Point, Rgba, Shape};
use tokio::time::sleep;

use crate::support::{Device, Rig, Setup, TempDir, eventually};

const ACCENT: Rgba = Rgba::hex(0x0A6CFF, 255);

/// 给一台设备配上边缘投递。常驻后台会把文件传输的进展转给边缘投递，这里由一个小任务代劳。
fn edge_drop_for(device: &Device) -> (Files, MockOverlay, Arc<EdgeDrop>) {
    let (events, mut rx) = Events::channel();
    let files = Files::new(&device.store, events);
    let overlay = MockOverlay::new();
    let edge = Arc::new(EdgeDrop::spawn(
        overlay.clone(),
        files.clone(),
        ACCENT,
        Lang::Zh,
    ));
    tokio::spawn({
        let edge = edge.clone();
        async move {
            while let Some(event) = rx.recv().await {
                if let Event::Transfer(update) = event {
                    edge.transfer(update);
                }
            }
        }
    });
    (files, overlay, edge)
}

/// 一台带着边缘投递的服务端（本机 1000×800，客户端在右侧），会话已经建好、光标在本机。
async fn rig_with_edge_drop() -> (Rig, MockOverlay, Arc<EdgeDrop>) {
    let mac = Device::new();
    let (files, overlay, edge) = edge_drop_for(&mac);
    let rig = Rig::with(Setup {
        mac,
        mac_files: Some(files),
        edge: edge.feed(),
        ..Setup::default()
    })
    .await;
    // 先切过去再回来：确认会话已经建好，光标回到本机。
    rig.enter().await;
    rig.leave().await;
    (rig, overlay, edge)
}

/// 两端都带着边缘投递：服务端 1000×800，客户端 1920×1080（150%）在它右侧。
struct Pair {
    rig: Rig,
    mac: MockOverlay,
    pc: MockOverlay,
    _edges: [Arc<EdgeDrop>; 2],
}

async fn pair_with_edge_drop() -> Pair {
    let (mac, pc) = (Device::new(), Device::new());
    let (mac_files, mac_overlay, mac_edge) = edge_drop_for(&mac);
    let (pc_files, pc_overlay, pc_edge) = edge_drop_for(&pc);
    let rig = Rig::with(Setup {
        mac,
        pc,
        mac_files: Some(mac_files),
        pc_files: Some(pc_files),
        edge: mac_edge.feed(),
        pc_edge: pc_edge.feed(),
        ..Setup::default()
    })
    .await;
    rig.enter().await;
    rig.leave().await;
    Pair {
        rig,
        mac: mac_overlay,
        pc: pc_overlay,
        _edges: [mac_edge, pc_edge],
    }
}

/// 胶囊沿竖直方向的中心。
fn middle_y(overlay: &MockOverlay) -> f32 {
    overlay
        .shown()
        .map_or(f32::NAN, |(frame, _)| frame.y + frame.h / 2.0)
}

fn source_file(dir: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    fs::create_dir_all(dir.path()).unwrap();
    let file = dir.path().join(name);
    fs::write(&file, bytes).unwrap();
    file
}

fn texts(overlay: &MockOverlay) -> Vec<String> {
    overlay.shown().map_or_else(Vec::new, |(_, scene)| {
        scene
            .shapes
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    })
}

fn pointer(rig: &Rig, x: i32, y: i32) {
    rig.capture.emit(CaptureEvent::Pointer {
        position: Point::new(x, y),
        dx: 1,
        dy: 0,
    });
}

fn left_button(rig: &Rig, pressed: bool) {
    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed,
    });
}

/// 按下鼠标，再开始拖 `files`。真机上系统要等鼠标移动几个像素才开始拖，按下总是先被看到；
/// 这里等一下，模拟同样的先后。
async fn press_and_drag(rig: &Rig, overlay: &MockOverlay, files: Option<Vec<std::path::PathBuf>>) {
    left_button(rig, true);
    sleep(Duration::from_millis(50)).await;
    if let Some(files) = files {
        overlay.start_drag(files);
    }
}

#[tokio::test]
async fn dragging_files_to_the_edge_shows_the_capsule_and_dropping_sends_them() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let source = TempDir::new();
    let file = source.path().join("证书.p12");
    fs::create_dir_all(source.path()).unwrap();
    fs::write(&file, b"secret").unwrap();

    // 按下鼠标，开始拖文件，再拖向右边缘（对方在右侧）。
    pointer(&rig, 500, 400);
    press_and_drag(&rig, &overlay, Some(vec![file.clone()])).await;
    eventually("靠近边缘时滑出胶囊", || {
        pointer(&rig, 900, 400);
        overlay.shown().is_some()
    })
    .await;
    let (window, _) = overlay.shown().unwrap();
    assert!(window.right() > 900.0, "贴着右边缘：{window:?}");

    // 拖到胶囊上：展开成“松手发送”。
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("展开成松手发送", || {
        texts(&overlay).contains(&"松手发送到".to_owned())
    })
    .await;
    assert!(texts(&overlay).contains(&"证书.p12".to_owned()));

    overlay.emit(OverlayEvent::Dropped {
        paths: vec![file.clone()],
    });
    left_button(&rig, false);
    let got = rig.pc.store.downloads_dir().join("证书.p12");
    eventually("对方收到文件", || got.is_file()).await;
    assert_eq!(fs::read(&got).unwrap(), b"secret");
    eventually("胶囊显示已发送", || {
        texts(&overlay).contains(&"已发送".to_owned())
    })
    .await;
}

#[tokio::test]
async fn a_received_file_shows_a_card_that_opens_it() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let source = TempDir::new();
    let file = source.path().join("会议纪要.docx");
    fs::create_dir_all(source.path()).unwrap();
    fs::write(&file, b"notes").unwrap();

    // 等会话建好、边缘报告给了边缘投递，再从客户端发过来。
    let mut sent = false;
    for _ in 0..300 {
        if let Some(peer) = rig.pc_files.connected().first().copied()
            && rig.pc_files.send(peer, vec![file.clone()]).await.is_ok()
        {
            sent = true;
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert!(sent, "客户端发出了文件");

    eventually("收完变成卡片", || {
        texts(&overlay).contains(&"打开".to_owned())
    })
    .await;
    assert!(texts(&overlay).contains(&"会议纪要.docx".to_owned()));
    // 点“打开”之前卡片要先完全出来，按钮才能点。
    sleep(Duration::from_millis(600)).await;
    let (_, scene) = overlay.shown().unwrap();
    assert!(scene.hits.iter().any(|h| h.id == hit::OPEN));

    overlay.emit(OverlayEvent::Clicked(hit::OPEN));
    let expected = rig.mac.store.downloads_dir().join("会议纪要.docx");
    eventually("用默认程序打开收到的文件", || {
        overlay.opened() == vec![expected.clone()]
    })
    .await;
    eventually("打开后卡片收起", || overlay.shown().is_none()).await;
}

#[tokio::test]
async fn dragging_something_other_than_files_shows_nothing() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    // 先确认拖文件时会出现胶囊，后面“没出现”才有意义。
    let source = TempDir::new();
    fs::create_dir_all(source.path()).unwrap();
    let file = source.path().join("a.txt");
    fs::write(&file, b"a").unwrap();
    press_and_drag(&rig, &overlay, Some(vec![file])).await;
    eventually("拖文件时出现胶囊", || {
        pointer(&rig, 900, 400);
        overlay.shown().is_some()
    })
    .await;
    left_button(&rig, false);
    eventually("松开后收起", || overlay.shown().is_none()).await;

    // 拖的是文字：拖放剪贴板变了，但里面没有文件。
    press_and_drag(&rig, &overlay, Some(Vec::new())).await;
    for x in [700, 850, 950, 999] {
        pointer(&rig, x, 400);
        sleep(Duration::from_millis(30)).await;
    }
    assert!(overlay.shown().is_none(), "拖文字不出现胶囊");
    left_button(&rig, false);

    // 拖窗口：拖放剪贴板根本没变。
    press_and_drag(&rig, &overlay, None).await;
    for x in [700, 850, 950, 999] {
        pointer(&rig, x, 400);
        sleep(Duration::from_millis(30)).await;
    }
    assert!(overlay.shown().is_none(), "拖窗口不出现胶囊");
    left_button(&rig, false);
}

#[tokio::test]
async fn without_a_drag_clipboard_the_edge_is_watched_until_files_come_in() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let source = TempDir::new();
    fs::create_dir_all(source.path()).unwrap();
    let file = source.path().join("素材.zip");
    fs::write(&file, b"zip").unwrap();

    // 像 Windows 那样：按下鼠标靠近边缘时，还不知道拖的是什么。
    press_and_drag(&rig, &overlay, None).await;
    eventually("在贴边留意", || {
        pointer(&rig, 960, 400);
        overlay.watching().is_some()
    })
    .await;
    let strip = overlay.watching().unwrap();
    assert!(
        strip.x >= 990.0 && strip.right() <= 1000.5,
        "贴着右边缘的一窄条：{strip:?}"
    );
    assert!(overlay.shown().is_none(), "还不知道是文件时不出现胶囊");

    // 拖着文件进了接收条：直接展开成“松手发送”，不再留意。
    overlay.start_drag(vec![file]);
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("展开成松手发送", || {
        texts(&overlay).contains(&"松手发送到".to_owned())
    })
    .await;
    eventually("不再留意", || overlay.watching().is_none()).await;
    left_button(&rig, false);
}

/// 一台带着边缘投递的客户端（1920×1080，服务端在它的左侧），会话已经建好。
async fn client_with_edge_drop() -> (Rig, MockOverlay, Arc<EdgeDrop>) {
    let pc = Device::new();
    let (files, overlay, edge) = edge_drop_for(&pc);
    let rig = Rig::with(Setup {
        pc,
        pc_files: Some(files),
        pc_edge: edge.feed(),
        ..Setup::default()
    })
    .await;
    rig.enter().await;
    rig.leave().await;
    (rig, overlay, edge)
}

#[tokio::test]
async fn the_client_learns_its_edge_from_the_server_and_shows_received_files() {
    let (rig, overlay, _edge) = client_with_edge_drop().await;
    let source = TempDir::new();
    fs::create_dir_all(source.path()).unwrap();
    let file = source.path().join("季度报告.pdf");
    fs::write(&file, b"pdf").unwrap();

    let mut peer = None;
    eventually("服务端的数据连接建好", || {
        peer = rig.mac_files.connected().first().copied();
        peer.is_some()
    })
    .await;
    rig.mac_files
        .send(peer.unwrap(), vec![file])
        .await
        .expect("服务端发出文件");
    eventually("客户端收完变成卡片", || {
        texts(&overlay).contains(&"打开".to_owned())
    })
    .await;
    let (window, _) = overlay.shown().unwrap();
    assert!(
        window.x < 100.0,
        "服务端在客户端左侧，卡片贴左边缘：{window:?}"
    );
}

#[tokio::test]
async fn dragging_with_the_clients_own_mouse_is_noticed_at_the_edge() {
    // 光标不归服务端控制：客户端的用户用自己的鼠标（或触控板）拖文件。
    let (rig, overlay, _edge) = client_with_edge_drop().await;
    let source = TempDir::new();
    fs::create_dir_all(source.path()).unwrap();
    let file = source.path().join("照片.jpg");
    fs::write(&file, b"jpg").unwrap();

    rig.inject
        .set_local_pointer(Some((Point::new(900, 400), false)));
    sleep(Duration::from_millis(250)).await;
    assert!(overlay.watching().is_none(), "没按键时不留意");

    // 按下鼠标，拖到朝向服务端的左边缘附近：像 Windows 那样先在贴边留意。
    rig.inject
        .set_local_pointer(Some((Point::new(900, 400), true)));
    sleep(Duration::from_millis(250)).await;
    rig.inject
        .set_local_pointer(Some((Point::new(40, 400), true)));
    eventually("在贴边留意", || overlay.watching().is_some()).await;

    // 拖着文件进了接收条：展开成“松手发送”。
    overlay.start_drag(vec![file]);
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("展开成松手发送", || {
        texts(&overlay).contains(&"松手发送到".to_owned())
    })
    .await;

    // 松开后不再留意；服务端把光标推过来时，改由它说了算。
    rig.inject
        .set_local_pointer(Some((Point::new(900, 400), false)));
    eventually("不再留意", || overlay.watching().is_none()).await;
    rig.enter().await;
}

#[tokio::test]
async fn dragging_files_to_the_edge_on_the_client_sends_them_to_the_server() {
    let (rig, overlay, _edge) = client_with_edge_drop().await;
    let source = TempDir::new();
    fs::create_dir_all(source.path()).unwrap();
    let file = source.path().join("会议纪要.docx");
    fs::write(&file, b"notes").unwrap();

    // 光标进入客户端（从它的左边缘进来），按下鼠标：服务端把按键转给客户端注入。
    rig.enter().await;
    left_button(&rig, true);
    eventually("客户端在贴边留意", || {
        rig.push(-1, 0);
        overlay.watching().is_some()
    })
    .await;

    // 拖着文件推到左边缘，进了接收条。
    overlay.start_drag(vec![file.clone()]);
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("客户端展开成松手发送", || {
        texts(&overlay).contains(&"松手发送到".to_owned())
    })
    .await;
    overlay.emit(OverlayEvent::Dropped { paths: vec![file] });
    left_button(&rig, false);

    let got = rig.mac.store.downloads_dir().join("会议纪要.docx");
    eventually("服务端收到文件", || got.is_file()).await;
    assert_eq!(fs::read(&got).unwrap(), b"notes");
}

#[tokio::test]
async fn dragging_toward_the_edge_slides_out_the_receiving_capsule_on_the_other_side() {
    let Pair { rig, mac, pc, .. } = pair_with_edge_drop().await;
    let source = TempDir::new();
    let file = source_file(&source, "设计稿.fig", &[7; 2048]);

    // 在本机 1/4 高度处拖向右边缘：对方在它左边缘的 1/4 高度处（1080 × 1/4 = 270）提前滑出胶囊。
    pointer(&rig, 500, 200);
    press_and_drag(&rig, &mac, Some(vec![file.clone()])).await;
    eventually("对方提前滑出接收胶囊", || {
        pointer(&rig, 900, 200);
        pc.shown().is_some()
    })
    .await;
    let (window, _) = pc.shown().unwrap();
    assert!(window.x < 100.0, "贴着对方的左边缘：{window:?}");
    eventually("在对应的高度", || {
        (middle_y(&pc) - 270.0).abs() < 20.0
    })
    .await;

    // 拖到本机胶囊上：对方展开成“松手即接收”，并说明是什么。
    mac.emit(OverlayEvent::DragEntered { files: true });
    eventually("对方展开成松手即接收", || {
        texts(&pc).contains(&"松手即接收".to_owned())
    })
    .await;
    let shown = texts(&pc);
    assert!(shown.contains(&"来自 mac".to_owned()), "{shown:?}");
    assert!(
        shown.contains(&"设计稿.fig · 2.0 KB".to_owned()),
        "{shown:?}"
    );

    // 放下：对方在同一高度显示收到的卡片。
    mac.emit(OverlayEvent::Dropped {
        paths: vec![file.clone()],
    });
    left_button(&rig, false);
    let got = rig.pc.store.downloads_dir().join("设计稿.fig");
    eventually("对方收到文件", || got.is_file()).await;
    eventually("对方显示收到的卡片", || {
        texts(&pc).contains(&"打开".to_owned())
    })
    .await;
    eventually("卡片在同一高度", || {
        (middle_y(&pc) - 270.0).abs() < 20.0
    })
    .await;
}

#[tokio::test]
async fn moving_away_from_the_edge_hides_the_receiving_capsule() {
    let Pair { rig, mac, pc, .. } = pair_with_edge_drop().await;
    let source = TempDir::new();
    let file = source_file(&source, "a.txt", b"a");

    pointer(&rig, 500, 400);
    press_and_drag(&rig, &mac, Some(vec![file])).await;
    eventually("对方提前滑出接收胶囊", || {
        pointer(&rig, 900, 400);
        pc.shown().is_some()
    })
    .await;

    // 拖回屏幕中间：两边都收起。
    eventually("本机收起", || {
        pointer(&rig, 500, 400);
        mac.shown().is_none()
    })
    .await;
    eventually("对方也收起", || pc.shown().is_none()).await;

    // 再靠近又出现；这次直接松开鼠标（取消拖动），对方同样收起。
    eventually("对方再次滑出", || {
        pointer(&rig, 900, 400);
        pc.shown().is_some()
    })
    .await;
    left_button(&rig, false);
    eventually("松开后对方收起", || pc.shown().is_none()).await;
}

#[tokio::test]
async fn dragging_on_the_client_slides_out_the_receiving_capsule_on_the_server() {
    let Pair { rig, mac, pc, .. } = pair_with_edge_drop().await;
    let source = TempDir::new();
    let file = source_file(&source, "照片.heic", b"heic");

    // 光标在客户端屏幕中间高度推向左边缘，拖着文件进了接收条。
    rig.enter().await;
    left_button(&rig, true);
    eventually("客户端在贴边留意", || {
        rig.push(-1, 0);
        pc.watching().is_some()
    })
    .await;
    pc.start_drag(vec![file]);
    pc.emit(OverlayEvent::DragEntered { files: true });

    // 服务端在右边缘的中间高度（800 × 1/2 = 400）展开成“松手即接收”。
    eventually("服务端展开成松手即接收", || {
        texts(&mac).contains(&"松手即接收".to_owned())
    })
    .await;
    assert!(texts(&mac).contains(&"来自 pc".to_owned()));
    let (window, _) = mac.shown().unwrap();
    assert!(window.right() > 900.0, "贴着服务端的右边缘：{window:?}");
    eventually("在对应的高度", || {
        (middle_y(&mac) - 400.0).abs() < 20.0
    })
    .await;

    pc.emit(OverlayEvent::DragExited);
    left_button(&rig, false);
}

#[tokio::test]
async fn after_the_peer_disconnects_dragging_files_shows_not_connected() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let source = TempDir::new();
    let file = source_file(&source, "a.txt", b"a");

    // 对方断开。之后照样能拖文件到边缘，只是显示“未连接”。
    rig.client.abort();
    pointer(&rig, 500, 400);
    press_and_drag(&rig, &overlay, Some(vec![file.clone()])).await;
    eventually("靠近边缘时滑出胶囊", || {
        pointer(&rig, 900, 400);
        overlay.shown().is_some()
    })
    .await;
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("显示未连接", || {
        texts(&overlay).contains(&"连上后才能发送".to_owned())
    })
    .await;
    assert!(texts(&overlay).contains(&"未连接".to_owned()));

    // 放下也发不出去，说明原因。
    overlay.emit(OverlayEvent::Dropped { paths: vec![file] });
    left_button(&rig, false);
    eventually("提示发不出去", || {
        texts(&overlay).contains(&"发不出去".to_owned())
    })
    .await;
}

/// 从客户端发一个文件过来，等服务端的胶囊变成卡片，返回保存的位置。
async fn receive_card(rig: &Rig, overlay: &MockOverlay, name: &str) -> std::path::PathBuf {
    let source = TempDir::new();
    let file = source_file(&source, name, b"notes");
    let mut sent = false;
    for _ in 0..300 {
        if let Some(peer) = rig.pc_files.connected().first().copied()
            && rig.pc_files.send(peer, vec![file.clone()]).await.is_ok()
        {
            sent = true;
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert!(sent, "客户端发出了文件");
    eventually("收完变成卡片", || {
        texts(overlay).contains(&"打开".to_owned())
    })
    .await;
    // 卡片完全出来后，可以拖的地方才在。
    sleep(Duration::from_millis(600)).await;
    rig.mac.store.downloads_dir().join(name)
}

#[tokio::test]
async fn the_received_files_can_be_dragged_out_of_the_card() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let saved = receive_card(&rig, &overlay, "会议纪要.docx").await;

    assert!(texts(&overlay).contains(&"按住这里，拖到任意文件夹".to_owned()));
    let (_, scene) = overlay.shown().unwrap();
    assert!(scene.hits.iter().any(|h| h.drag && h.id == hit::CHIP));
    eventually("把收到的文件交给浮层", || {
        overlay.offered() == vec![saved.clone()]
    })
    .await;

    // 拖出去的路上经过了浮层自己（它就贴着边缘）：不当作要发给对方。
    overlay.emit(OverlayEvent::DragOutStarted);
    press_and_drag(&rig, &overlay, Some(vec![saved.clone()])).await;
    pointer(&rig, 990, 400);
    overlay.emit(OverlayEvent::DragEntered { files: true });
    overlay.emit(OverlayEvent::Dropped {
        paths: vec![saved.clone()],
    });
    sleep(Duration::from_millis(300)).await;
    let shown = texts(&overlay);
    assert!(shown.contains(&"打开".to_owned()), "卡片还在：{shown:?}");
    assert!(!shown.contains(&"松手发送到".to_owned()));

    // 放到了别处：卡片收起，不再交出文件。
    overlay.emit(OverlayEvent::DragOutEnded { dropped: true });
    left_button(&rig, false);
    eventually("放下后卡片收起", || overlay.shown().is_none()).await;
    eventually("不再交出文件", || overlay.offered().is_empty()).await;
    assert!(saved.is_file(), "引擎不碰文件本身");
}

#[tokio::test]
async fn cancelling_a_drag_out_keeps_the_card() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let saved = receive_card(&rig, &overlay, "照片.heic").await;

    overlay.emit(OverlayEvent::DragOutStarted);
    overlay.emit(OverlayEvent::DragOutEnded { dropped: false });
    sleep(Duration::from_millis(300)).await;
    assert!(
        texts(&overlay).contains(&"打开".to_owned()),
        "取消后卡片还在"
    );
    assert_eq!(overlay.offered(), vec![saved]);
}

#[tokio::test]
async fn dragging_the_cards_own_files_toward_the_edge_does_not_send_them_back() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    let saved = receive_card(&rig, &overlay, "设计稿.fig").await;

    // 拖动已经开始，浮层还没来得及说“开始往外拖”：拖的正是卡片里的文件，也不当作要发送。
    pointer(&rig, 980, 400);
    press_and_drag(&rig, &overlay, Some(vec![saved])).await;
    for x in [985, 990, 995] {
        pointer(&rig, x, 400);
        sleep(Duration::from_millis(30)).await;
    }
    let shown = texts(&overlay);
    assert!(shown.contains(&"打开".to_owned()), "卡片还在：{shown:?}");
    left_button(&rig, false);
}

#[tokio::test]
async fn with_two_clients_on_one_edge_files_go_to_the_one_whose_part_they_are_dropped_on() {
    let (rig, overlay, _edge) = rig_with_edge_drop().await;
    // 两台都在右边：先连上的 pc 在上半段，laptop 在下半段。
    let laptop = rig.add_client("laptop", EdgeFeed::none()).await;
    let source = TempDir::new();
    let file = source_file(&source, "给笔记本.txt", b"laptop");

    pointer(&rig, 500, 700);
    press_and_drag(&rig, &overlay, Some(vec![file.clone()])).await;
    eventually("下半段滑出胶囊", || {
        pointer(&rig, 900, 700);
        overlay.shown().is_some_and(|(window, _)| window.y > 400.0)
    })
    .await;
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("写着发给 laptop", || {
        let shown = texts(&overlay);
        shown.contains(&"松手发送到".to_owned()) && shown.contains(&"laptop".to_owned())
    })
    .await;

    overlay.emit(OverlayEvent::Dropped {
        paths: vec![file.clone()],
    });
    left_button(&rig, false);
    let got = laptop.device.store.downloads_dir().join("给笔记本.txt");
    eventually("laptop 收到文件", || got.is_file()).await;
    assert!(
        !rig.pc.store.downloads_dir().join("给笔记本.txt").exists(),
        "pc 没有收到"
    );

    // 上半段是 pc。
    pointer(&rig, 500, 100);
    press_and_drag(&rig, &overlay, Some(vec![file])).await;
    overlay.emit(OverlayEvent::DragEntered { files: true });
    eventually("上半段写着发给 pc", || {
        pointer(&rig, 900, 100);
        let shown = texts(&overlay);
        shown.contains(&"松手发送到".to_owned()) && shown.contains(&"pc".to_owned())
    })
    .await;
    left_button(&rig, false);
}
