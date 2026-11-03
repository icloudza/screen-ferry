use ferry_platform::CaptureEvent;
use ferry_types::{InputEvent, KeyAction, KeyCode, MouseButton, Point};

use crate::support::{EDGE, ENTRY, RETURN, Rig, Setup, display, eventually};

fn motion(x: i32, y: i32) -> InputEvent {
    InputEvent::PointerMotion {
        position: Point::new(x, y),
    }
}

#[tokio::test]
async fn cursor_crosses_to_the_client_and_back() {
    let rig = Rig::start().await;
    rig.enter().await;

    rig.push(10, 0); // 150% 缩放：1 + 15 = 16
    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: true,
    });
    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: false,
    });
    rig.push(-20, 0); // 16 - 30 = -14：从左边缘推出去 14

    eventually("回到本机", || !rig.capture.is_grabbed()).await;
    assert_eq!(
        rig.capture.last_release(),
        Some(Point::new(991, 400)),
        "推出去的 14 换算回本机是 9，落在第 9 个单位；纵坐标换算回 400",
    );

    let at = Point::new(16, 540);
    eventually("客户端收到全部事件", || {
        rig.inject.events().len() >= 4
    })
    .await;
    assert_eq!(
        rig.inject.events(),
        [
            motion(ENTRY.x, ENTRY.y),
            motion(at.x, at.y),
            InputEvent::Button {
                button: MouseButton::LEFT,
                pressed: true,
                position: at
            },
            InputEvent::Button {
                button: MouseButton::LEFT,
                pressed: false,
                position: at
            },
        ],
    );
}

#[tokio::test]
async fn the_configured_edge_delay_applies_in_both_directions() {
    use std::time::Duration;

    use ferry_engine::Live;
    use ferry_types::DeviceSettings;

    let rig = Rig::with(Setup {
        live: Some(
            Live {
                side: ferry_core::Side::Right,
                layout: Vec::new(),
                devices: Vec::new(),
                fallback: DeviceSettings {
                    edge_delay_ms: 500,
                    ..DeviceSettings::default()
                },
                mouse_dpi: 1600,
            }
            .fixed(),
        ),
        ..Setup::default()
    })
    .await;
    rig.enter().await;
    rig.push(-50, 0);
    eventually("返回前停在客户端边缘", || {
        rig.inject.events().contains(&motion(0, ENTRY.y))
    })
    .await;
    assert!(rig.capture.is_grabbed(), "刚到返回边缘不能立即切回");
    tokio::time::sleep(Duration::from_millis(550)).await;
    rig.push(-1, 0);
    eventually("等待后推出边缘回到本机", || {
        !rig.capture.is_grabbed()
    })
    .await;
}

#[tokio::test]
async fn holding_a_button_keeps_the_cursor_on_the_client() {
    let rig = Rig::start().await;
    rig.enter().await;

    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: true,
    });
    rig.push(-500, 0);
    eventually("客户端收到按下", || rig.inject.events().len() >= 2).await;
    assert!(rig.capture.is_grabbed(), "拖拽中不能切回本机");

    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: false,
    });
    rig.push(-1, 0);
    eventually("松开后可以回到本机", || !rig.capture.is_grabbed()).await;
}

#[tokio::test]
async fn control_returns_when_the_client_disconnects() {
    let rig = Rig::start().await;
    rig.enter().await;

    rig.client.abort();
    eventually("客户端断开后交还控制", || {
        !rig.capture.is_grabbed()
    })
    .await;
    assert_eq!(rig.capture.last_release(), Some(RETURN));
}

#[tokio::test]
async fn input_from_before_a_switch_is_not_replayed_after_it() {
    let rig = Rig::start().await;
    rig.enter().await;

    // 进入之前已交给本机的移动和滚轮，此刻才送到：不能再当作对方屏幕上的输入。
    rig.capture.emit_stale(CaptureEvent::Pointer {
        position: EDGE,
        dx: -300,
        dy: 0,
    });
    rig.capture
        .emit_stale(CaptureEvent::Wheel { dx: 0, dy: 120 });
    rig.push(10, 0);
    eventually("客户端收到移动", || rig.inject.events().len() >= 2).await;
    assert!(rig.capture.is_grabbed(), "过时的移动没有把光标带回本机");
    assert_eq!(
        rig.inject.events(),
        [motion(ENTRY.x, ENTRY.y), motion(16, 540)],
        "过时的滚轮没有转发",
    );

    rig.leave().await;
    // 离开之前被吞掉的移动，此刻才送到：它带着过时的位置，不能拿它判断边缘、又切回去。
    rig.capture.emit_stale(CaptureEvent::Pointer {
        position: EDGE,
        dx: 30,
        dy: 0,
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!rig.capture.is_grabbed(), "过时的移动没有让光标再切过去");
}

#[tokio::test]
async fn a_client_display_change_mid_session_is_followed() {
    let rig = Rig::start().await;
    rig.enter().await;

    // 客户端把分辨率改小：服务端随后按新的范围移动光标，右边缘从 1919 变成 1279。
    rig.pc_displays.set(vec![display(1280, 720, 150)]);
    eventually("服务端按新的范围移动光标", || {
        rig.push(-10, 0);
        rig.push(2000, 0);
        matches!(
            rig.inject.events().last(),
            Some(InputEvent::PointerMotion { position }) if position.x == 1279
        )
    })
    .await;
}

#[tokio::test]
async fn a_server_display_change_mid_session_is_followed() {
    let rig = Rig::start().await;
    // 先完整进出一次，确认会话已经在进行。
    rig.enter().await;
    rig.leave().await;

    // 服务端把分辨率改小：右边缘从 999 变成 799。
    rig.mac_displays.set(vec![display(800, 600, 100)]);
    eventually("从新的右边缘进入", || {
        if rig.capture.is_grabbed() {
            return true;
        }
        rig.capture.emit(CaptureEvent::Pointer {
            position: Point::new(799, 300),
            dx: 1,
            dy: 0,
        });
        false
    })
    .await;

    rig.leave().await;
    let landed = rig.capture.last_release().expect("回到本机时放好了光标");
    assert!(
        landed.x < 800 && landed.y < 600,
        "落在新的范围内：{landed:?}"
    );
}

fn key(code: KeyCode, action: KeyAction) -> InputEvent {
    InputEvent::Key { code, action }
}

#[tokio::test]
async fn keys_are_forwarded_with_command_as_control() {
    let rig = Rig::start().await;
    let letter_c = KeyCode::keyboard(0x06);

    rig.key(letter_c, KeyAction::Down);
    rig.key(letter_c, KeyAction::Up);
    rig.enter().await;
    rig.key(KeyCode::LEFT_GUI, KeyAction::Down);
    rig.key(letter_c, KeyAction::Down);
    rig.key(letter_c, KeyAction::Repeat);
    rig.key(letter_c, KeyAction::Up);
    rig.key(KeyCode::LEFT_GUI, KeyAction::Up);

    eventually("客户端收到按键", || rig.inject.events().len() >= 6).await;
    assert_eq!(
        rig.inject.events(),
        [
            motion(ENTRY.x, ENTRY.y),
            key(KeyCode::LEFT_CTRL, KeyAction::Down),
            key(letter_c, KeyAction::Down),
            key(letter_c, KeyAction::Repeat),
            key(letter_c, KeyAction::Up),
            key(KeyCode::LEFT_CTRL, KeyAction::Up),
        ],
        "光标进入前的按键不转发；⌘ 换成 Ctrl",
    );
}

#[tokio::test]
async fn two_machines_with_the_same_kind_of_keyboard_keep_command_and_control() {
    let rig = Rig::with(Setup {
        pc_command_key: true,
        ..Setup::default()
    })
    .await;
    rig.enter().await;
    rig.key(KeyCode::LEFT_GUI, KeyAction::Down);
    rig.key(KeyCode::LEFT_CTRL, KeyAction::Down);

    eventually("客户端收到按键", || rig.inject.events().len() >= 3).await;
    assert_eq!(
        rig.inject.events()[1..],
        [
            key(KeyCode::LEFT_GUI, KeyAction::Down),
            key(KeyCode::LEFT_CTRL, KeyAction::Down),
        ],
        "两台都是 Mac：⌘ 还是 ⌘",
    );
}

#[tokio::test]
async fn a_pc_controlling_a_mac_sends_control_as_command() {
    let rig = Rig::with(Setup {
        mac_command_key: false,
        pc_command_key: true,
        ..Setup::default()
    })
    .await;
    let letter_c = KeyCode::keyboard(0x06);
    rig.enter().await;
    rig.key(KeyCode::LEFT_CTRL, KeyAction::Down);
    rig.key(letter_c, KeyAction::Down);

    eventually("客户端收到按键", || rig.inject.events().len() >= 3).await;
    assert_eq!(
        rig.inject.events()[1..],
        [
            key(KeyCode::LEFT_GUI, KeyAction::Down),
            key(letter_c, KeyAction::Down),
        ],
        "Windows 控制 Mac：Ctrl + C 到了 Mac 上是 ⌘C",
    );
}

#[tokio::test]
async fn media_keys_are_forwarded_and_released_on_leave() {
    let rig = Rig::start().await;
    rig.enter().await;
    rig.key(KeyCode::VOLUME_UP, KeyAction::Down);
    rig.key(KeyCode::VOLUME_UP, KeyAction::Repeat);
    rig.key(KeyCode::VOLUME_UP, KeyAction::Up);
    rig.key(KeyCode::PLAY_PAUSE, KeyAction::Down);
    rig.leave().await; // 播放键还按着就离开了

    eventually("客户端收到媒体键并松开", || {
        rig.inject.events().len() >= 6
    })
    .await;
    let keys: Vec<_> = rig
        .inject
        .events()
        .into_iter()
        .filter(|e| matches!(e, InputEvent::Key { .. }))
        .collect();
    assert_eq!(
        keys,
        [
            key(KeyCode::VOLUME_UP, KeyAction::Down),
            key(KeyCode::VOLUME_UP, KeyAction::Repeat),
            key(KeyCode::VOLUME_UP, KeyAction::Up),
            key(KeyCode::PLAY_PAUSE, KeyAction::Down),
            key(KeyCode::PLAY_PAUSE, KeyAction::Up),
        ],
        "媒体键原样转发，离开时松开",
    );
}

#[tokio::test]
async fn ctrl_option_escape_returns_to_local() {
    let rig = Rig::start().await;
    rig.enter().await;

    rig.key(KeyCode::LEFT_CTRL, KeyAction::Down);
    rig.key(KeyCode::RIGHT_ALT, KeyAction::Down);
    rig.key(KeyCode::ESCAPE, KeyAction::Down);

    eventually("快捷键回到本机", || !rig.capture.is_grabbed()).await;
    assert_eq!(rig.capture.last_release(), Some(RETURN));

    // 客户端收到 Leave 后松开按着的修饰键（Ctrl 已被映射成 Win），Esc 本身不转发。
    eventually("客户端松开修饰键", || {
        rig.inject.events().len() >= 5
    })
    .await;
    assert_eq!(
        rig.inject.events()[1..],
        [
            key(KeyCode::LEFT_GUI, KeyAction::Down),
            key(KeyCode::RIGHT_ALT, KeyAction::Down),
            key(KeyCode::LEFT_GUI, KeyAction::Up),
            key(KeyCode::RIGHT_ALT, KeyAction::Up),
        ],
    );
}

#[tokio::test]
async fn changing_side_mid_session_takes_effect_without_reconnecting() {
    use ferry_core::Side;
    use ferry_engine::Live;
    use tokio::sync::watch;

    use crate::support::Setup;

    let (tx, rx) = watch::channel(Live {
        side: Side::Right,
        layout: Vec::new(),
        devices: Vec::new(),
        fallback: ferry_types::DeviceSettings::default(),
        mouse_dpi: 1600,
    });
    let rig = Rig::with(Setup {
        live: Some(rx),
        ..Setup::default()
    })
    .await;
    rig.enter().await;

    // 光标在对方屏幕时改方向：先回到本机。
    tx.send_modify(|live| live.side = Side::Left);
    eventually("改方向后回到本机", || !rig.capture.is_grabbed()).await;

    // 右边缘不再切换，左边缘可以。
    for _ in 0..5 {
        rig.push(5, 0);
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!rig.capture.is_grabbed(), "右边缘已经不通了");
    eventually("从左边缘进入", || {
        if rig.capture.is_grabbed() {
            return true;
        }
        rig.capture.emit(CaptureEvent::Pointer {
            position: Point::new(0, 400),
            dx: -5,
            dy: 0,
        });
        false
    })
    .await;
}

mod several_clients {
    use ferry_core::Side;
    use ferry_engine::{EdgeFeed, Live};
    use ferry_types::Placement;
    use tokio::sync::watch;

    use crate::support::{Rig, Setup, eventually};

    /// 服务端（1000×800）连着 pc，`live` 由测试掌握。
    async fn rig() -> (Rig, watch::Sender<Live>) {
        let (tx, rx) = watch::channel(Live {
            side: Side::Right,
            layout: Vec::new(),
            devices: Vec::new(),
            fallback: ferry_types::DeviceSettings::default(),
            mouse_dpi: 1600,
        });
        let rig = Rig::with(Setup {
            live: Some(rx),
            ..Setup::default()
        })
        .await;
        rig.enter().await;
        rig.leave().await;
        (rig, tx)
    }

    /// 在 `(x, y)` 朝 `(dx, dy)` 往外推，直到进入某台客户端。
    async fn enter_at(rig: &Rig, x: i32, y: i32, dx: i32, dy: i32) {
        eventually("进入一台客户端", || {
            if rig.capture.is_grabbed() {
                return true;
            }
            rig.push_at(x, y, dx, dy);
            false
        })
        .await;
    }

    /// 朝 `dx` 方向一直推，直到回到本机。
    async fn come_back(rig: &Rig, dx: i32) {
        eventually("回到本机", || {
            if !rig.capture.is_grabbed() {
                return true;
            }
            rig.push_at(500, 400, dx, 0);
            false
        })
        .await;
    }

    #[tokio::test]
    async fn each_client_gets_the_edge_it_is_placed_on() {
        let (rig, live) = rig().await;
        let laptop = rig.add_client("laptop", EdgeFeed::none()).await;
        let fingerprint = laptop.fingerprint();
        live.send_modify(|l| {
            l.layout = vec![Placement {
                fingerprint,
                side: Side::Left,
            }]
        });

        // 左边缘进 laptop，pc 收不到任何东西。
        let pc_before = rig.inject.events().len();
        enter_at(&rig, 0, 400, -4, 0).await;
        eventually("laptop 收到进入", || !laptop.inject.events().is_empty()).await;
        assert_eq!(rig.inject.events().len(), pc_before, "pc 没有收到");
        come_back(&rig, 50).await;

        // 右边缘仍然是 pc。
        let laptop_before = laptop.inject.events().len();
        enter_at(&rig, 999, 400, 4, 0).await;
        eventually("pc 收到进入", || rig.inject.events().len() > pc_before).await;
        assert_eq!(
            laptop.inject.events().len(),
            laptop_before,
            "laptop 没有收到"
        );
    }

    #[tokio::test]
    async fn clients_on_the_same_side_share_the_edge_and_one_leaving_frees_its_part() {
        let (rig, live) = rig().await;
        let laptop = rig.add_client("laptop", EdgeFeed::none()).await;
        // 都在右边：pc 在上半段，laptop 在下半段。
        let (pc, lp) = (rig.pc_fingerprint(), laptop.fingerprint());
        live.send_modify(|l| {
            l.layout = vec![
                Placement {
                    fingerprint: pc,
                    side: Side::Right,
                },
                Placement {
                    fingerprint: lp,
                    side: Side::Right,
                },
            ]
        });

        enter_at(&rig, 999, 700, 4, 0).await;
        eventually("下半段进 laptop", || !laptop.inject.events().is_empty()).await;
        come_back(&rig, -50).await;

        let pc_before = rig.inject.events().len();
        enter_at(&rig, 999, 100, 4, 0).await;
        eventually("上半段进 pc", || rig.inject.events().len() > pc_before).await;
        come_back(&rig, -50).await;

        // laptop 断开：右边整条都归 pc。
        laptop.client.abort();
        let pc_before = rig.inject.events().len();
        eventually("下半段也进 pc", || {
            if rig.capture.is_grabbed() {
                return rig.inject.events().len() > pc_before;
            }
            rig.push_at(999, 700, 4, 0);
            false
        })
        .await;
    }

    #[tokio::test]
    async fn moving_the_client_under_the_cursor_brings_the_cursor_back_first() {
        let (rig, live) = rig().await;
        let _laptop = rig.add_client("laptop", EdgeFeed::none()).await;
        let pc = rig.pc_fingerprint();
        enter_at(&rig, 999, 100, 4, 0).await;

        live.send_modify(|l| {
            l.layout = vec![Placement {
                fingerprint: pc,
                side: Side::Top,
            }]
        });
        eventually("先回到本机", || !rig.capture.is_grabbed()).await;
        enter_at(&rig, 500, 0, 0, -4).await;
    }
}

/// 每台设备的单独设置（ADR 0009）：在服务端发出之前处理。
mod per_device {
    use std::time::Duration;

    use ferry_core::Side;
    use ferry_engine::{Event, Events, Live};
    use ferry_platform::CaptureEvent;
    use ferry_types::{
        DeviceSettings, InputEvent, KeyAction, KeyCode, KeyMapping, ModifierPreset, ScrollSpeed,
    };
    use tokio::sync::watch;

    use crate::support::{Rig, Setup, eventually};

    /// 服务端连着 pc，`live` 由测试掌握。
    async fn rig() -> (Rig, watch::Sender<Live>) {
        let (tx, rx) = watch::channel(Live {
            side: Side::Right,
            layout: Vec::new(),
            devices: Vec::new(),
            fallback: DeviceSettings::default(),
            mouse_dpi: 1600,
        });
        let rig = Rig::with(Setup {
            live: Some(rx),
            ..Setup::default()
        })
        .await;
        (rig, tx)
    }

    fn keys(rig: &Rig) -> Vec<(KeyCode, KeyAction)> {
        rig.inject
            .events()
            .into_iter()
            .filter_map(|e| match e {
                InputEvent::Key { code, action } => Some((code, action)),
                _ => None,
            })
            .collect()
    }

    fn wheels(rig: &Rig) -> Vec<(i32, i32)> {
        rig.inject
            .events()
            .into_iter()
            .filter_map(|e| match e {
                InputEvent::Wheel { dx, dy, .. } => Some((dx, dy)),
                _ => None,
            })
            .collect()
    }

    /// 光标在客户端上时的最后位置。
    fn last_x(rig: &Rig) -> i32 {
        rig.inject
            .events()
            .into_iter()
            .rev()
            .find_map(|e| match e {
                InputEvent::PointerMotion { position } => Some(position.x),
                _ => None,
            })
            .expect("客户端收到过光标位置")
    }

    #[tokio::test]
    async fn a_device_gets_its_own_wheel_and_keys() {
        let (rig, tx) = rig().await;
        let f13 = KeyCode::keyboard(0x68);
        let print_screen = KeyCode::keyboard(0x46);
        let mut device = DeviceSettings::new(rig.pc_fingerprint());
        device.reverse_vertical = true;
        device.scroll_speed = ScrollSpeed::Fast;
        device.modifiers = ModifierPreset::Plain;
        device.keys = vec![
            KeyMapping {
                from: f13.raw(),
                to: print_screen.raw(),
            },
            KeyMapping {
                from: KeyCode::CAPS_LOCK.raw(),
                to: 0,
            },
        ];
        tx.send_modify(|live| live.devices = vec![device]);
        rig.enter().await;

        rig.capture.emit(CaptureEvent::Wheel { dx: 0, dy: 120 });
        rig.key(KeyCode::LEFT_GUI, KeyAction::Down);
        rig.key(KeyCode::CAPS_LOCK, KeyAction::Down);
        rig.key(f13, KeyAction::Down);
        eventually("客户端收到按键", || keys(&rig).len() >= 2).await;

        assert_eq!(wheels(&rig), [(0, -240)], "反向、两倍");
        assert_eq!(
            keys(&rig),
            [
                (KeyCode::LEFT_GUI, KeyAction::Down),
                (print_screen, KeyAction::Down),
            ],
            "原样发送 ⌘；Caps Lock 不发送；F13 换成 Print Screen",
        );
    }

    #[tokio::test]
    async fn the_pointer_follows_the_dpi_set_for_the_device() {
        let (rig, tx) = rig().await;
        rig.enter().await;
        let start = last_x(&rig);
        rig.push(10, 0);
        eventually("光标在客户端上移动", || last_x(&rig) != start).await;
        let normal = last_x(&rig) - start;

        let mut device = DeviceSettings::new(rig.pc_fingerprint());
        device.pointer_dpi = 3200;
        tx.send_modify(|live| live.devices = vec![device]);
        // 等服务端换上新的设置。
        tokio::time::sleep(Duration::from_millis(100)).await;
        let before = last_x(&rig);
        rig.push(10, 0);
        eventually("光标在客户端上移动", || last_x(&rig) != before).await;
        assert_eq!(
            last_x(&rig) - before,
            normal * 2,
            "3200 DPI 是本机 1600 的两倍"
        );
    }

    #[tokio::test]
    async fn a_client_can_ask_the_server_to_open_its_settings() {
        let (events, mut rx) = Events::channel();
        let rig = Rig::with(Setup {
            mac_events: events,
            ..Setup::default()
        })
        .await;
        let (extra, commands) = rig.add_commanded_client("书房").await;
        commands
            .send(ferry_engine::ClientCommand::OpenPeerSettings)
            .unwrap();

        let fingerprint = extra.fingerprint();
        let asked = async {
            while let Some(event) = rx.recv().await {
                if let Event::OpenSettings { peer } = event
                    && peer.to_hex() == fingerprint
                {
                    return;
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(5), asked)
            .await
            .expect("服务端 5 秒内没有收到打开设置的请求");
    }
}
