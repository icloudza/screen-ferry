use ferry_engine::{ClipSummary, Event};
use ferry_types::{ClipboardContent, ClipboardFormat, KeyAction, KeyCode};

use crate::support::{Rig, Setup, collect, eventually, fake_png, has};

#[tokio::test]
async fn a_session_reports_its_lifecycle_on_both_sides() {
    let (mac_events, mac) = collect();
    let (pc_events, pc) = collect();
    let rig = Rig::with(Setup {
        mac_events,
        pc_events,
        mac_platform: "macos",
        pc_platform: "linux",
        ..Setup::default()
    })
    .await;

    eventually("服务端开始监听", || {
        has(&mac, |e| matches!(e, Event::Listening { .. }))
    })
    .await;
    eventually("两边都报告连上", || {
        has(
            &mac,
            |e| matches!(e, Event::Connected { name, .. } if name == "pc"),
        ) && has(
            &pc,
            |e| matches!(e, Event::Connected { name, .. } if name == "mac"),
        )
    })
    .await;
    eventually("两边都报告对端系统", || {
        has(
            &mac,
            |e| matches!(e, Event::PeerPlatform { platform, .. } if platform == "linux"),
        ) && has(
            &pc,
            |e| matches!(e, Event::PeerPlatform { platform, .. } if platform == "macos"),
        )
    })
    .await;
    assert!(has(&mac, |e| matches!(
        e,
        Event::Connected { platform: None, .. }
    )));
    assert!(has(&pc, |e| matches!(
        e,
        Event::Connected { platform: None, .. }
    )));
    // 首次连接要配对，两边都记下了对方。
    assert!(has(&mac, |e| matches!(e, Event::Paired { .. })));
    assert!(has(&pc, |e| matches!(e, Event::Paired { .. })));

    rig.enter().await;
    eventually("光标过去了", || {
        has(&mac, |e| {
            matches!(
                e,
                Event::Cursor {
                    here: false,
                    hotkey: false,
                    ..
                }
            )
        }) && has(&pc, |e| {
            matches!(
                e,
                Event::Cursor {
                    here: true,
                    hotkey: false,
                    ..
                }
            )
        })
    })
    .await;

    // 快捷键回来。
    rig.key(KeyCode::LEFT_CTRL, KeyAction::Down);
    rig.key(KeyCode::LEFT_ALT, KeyAction::Down);
    rig.key(KeyCode::ESCAPE, KeyAction::Down);
    eventually("按快捷键回到本机", || {
        has(&mac, |e| {
            matches!(
                e,
                Event::Cursor {
                    here: true,
                    hotkey: true,
                    ..
                }
            )
        }) && has(&pc, |e| {
            matches!(
                e,
                Event::Cursor {
                    here: false,
                    hotkey: false,
                    ..
                }
            )
        })
    })
    .await;

    eventually("两边都测到往返延迟", || {
        has(&mac, |e| matches!(e, Event::Rtt { .. }))
            && has(&pc, |e| matches!(e, Event::Rtt { .. }))
    })
    .await;

    // Mac 上复制，光标过去后 PC 拉取。
    rig.mac_clipboard.copy(ClipboardContent::text("你好，世界"));
    rig.enter().await;
    eventually("两边都报告剪贴板同步", || {
        has(&pc, |e| {
            matches!(e, Event::Clipboard { to_peer: false, .. })
        }) && has(&mac, |e| {
            matches!(e, Event::Clipboard { to_peer: true, .. })
        })
    })
    .await;
    assert!(has(&pc, |e| matches!(
        e,
        Event::Clipboard { summary, .. } if summary.preview == "你好，世界" && summary.chars == Some(5)
    )));

    rig.client.abort();
    eventually("服务端报告断开", || {
        has(&mac, |e| matches!(e, Event::Disconnected { .. }))
    })
    .await;
}

#[test]
fn clipboard_summary_counts_characters_and_flattens_the_preview() {
    let s = ClipSummary::of(&ClipboardContent::text("第一行\n第二行\t尾"));
    assert_eq!(s.chars, Some(9));
    assert_eq!(s.bytes, "第一行\n第二行\t尾".len() as u64);
    assert_eq!(s.preview, "第一行 第二行 尾");

    let long = "字".repeat(200);
    assert_eq!(
        ClipSummary::of(&ClipboardContent::text(long))
            .preview
            .chars()
            .count(),
        80
    );
}

#[test]
fn clipboard_summary_describes_images_and_rich_text() {
    let mut content = ClipboardContent::text("标题");
    content.insert(
        ClipboardFormat::HTML,
        b"<h1>\xe6\xa0\x87\xe9\xa2\x98</h1>".to_vec(),
    );
    content.insert(ClipboardFormat::PNG, fake_png(1920, 1080, 100));
    let s = ClipSummary::of(&content);
    assert!(s.rich);
    let image = s.image.expect("有图片");
    assert_eq!((image.width, image.height), (1920, 1080));
    assert_eq!(image.png.len(), fake_png(1920, 1080, 100).len());
    assert!(
        format!("{image:?}").contains("1920×1080"),
        "调试输出不打印整张图片"
    );

    let plain = ClipSummary::of(&ClipboardContent::text("x"));
    assert!(!plain.rich && plain.image.is_none());
}
