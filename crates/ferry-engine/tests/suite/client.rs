use ferry_platform::CaptureEvent;
use ferry_types::{InputEvent, LockKeys, MouseButton};

use crate::support::{ENTRY, Rig, eventually};

#[tokio::test]
async fn secure_input_crosses_real_tls_and_the_backend_never_reinjects_unsigned_events() {
    use crate::support::Setup;
    use ferry_platform_mock::MockInject;
    use ferry_proto::SecureAction;
    use ferry_trust::InputSigner;
    use ferry_types::{KeyAction, KeyCode};

    let setup = Setup::default();
    let signer = InputSigner::new(&setup.mac.store.identity("mac").unwrap()).unwrap();
    let inject = MockInject::new();
    inject.set_input_authorization(Some(signer.authorization().clone()));
    let rig = Rig::with(Setup {
        pc_inject: Some(inject),
        ..setup
    })
    .await;
    eventually("代理收到签署的 Enter", || {
        if !rig.capture.is_grabbed() {
            rig.push(1, 0);
        }
        rig.inject.verified_inputs().iter().any(|proof| {
            matches!(
                SecureAction::decode(&proof.body),
                Ok(SecureAction::Enter(_))
            )
        })
    })
    .await;
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Down);
    eventually("签署按键已经验证", || {
        rig.inject.verified_inputs().iter().any(|proof| {
            matches!(
                SecureAction::decode(&proof.body),
                Ok(SecureAction::Input(InputEvent::Key {
                    action: KeyAction::Down,
                    ..
                }))
            )
        })
    })
    .await;
    // 越过初始挑战的有效期：后续签署保活必须使用代理轮换出的新挑战。
    tokio::time::sleep(std::time::Duration::from_millis(1700)).await;
    assert!(rig.capture.is_grabbed());
    let proofs = rig.inject.verified_inputs();
    assert!(
        proofs
            .windows(2)
            .all(|pair| pair[1].sequence == pair[0].sequence + 1)
    );
    assert!(proofs.iter().any(|p| p.challenge != proofs[0].challenge));
    assert!(
        rig.inject.events().is_empty(),
        "后台不能把证明拆成未签署的再次注入"
    );
    assert!(
        rig.inject.synced_locks().is_empty(),
        "锁定键只能由代理执行签署的 Enter"
    );
    rig.client.abort();
    eventually("取消引擎也结束安全输入", || {
        rig.inject.ended_input_sessions() > 0
    })
    .await;
    eventually("控制端收回键鼠", || !rig.capture.is_grabbed()).await;
    assert!(rig.inject.events().is_empty());
}

#[tokio::test]
async fn a_broken_secure_input_service_falls_back_to_ordinary_input() {
    use crate::support::Setup;
    use ferry_platform_mock::MockInject;
    let inject = MockInject::new();
    inject.break_remote_challenge();
    let rig = Rig::with(Setup {
        pc_inject: Some(inject),
        ..Setup::default()
    })
    .await;
    rig.enter().await;
    rig.push(5, 0);
    eventually("普通权限的输入照常注入", || {
        !rig.inject.events().is_empty()
    })
    .await;
    assert!(rig.inject.verified_inputs().is_empty());
}

#[tokio::test]
async fn revoking_independent_input_authorization_ends_the_active_session() {
    use crate::support::Setup;
    use ferry_platform_mock::MockInject;
    use ferry_trust::InputSigner;
    let setup = Setup::default();
    let signer = InputSigner::new(&setup.mac.store.identity("mac").unwrap()).unwrap();
    let inject = MockInject::new();
    inject.set_input_authorization(Some(signer.authorization().clone()));
    let rig = Rig::with(Setup {
        pc_inject: Some(inject),
        ..setup
    })
    .await;
    eventually("建立安全输入会话", || {
        if !rig.capture.is_grabbed() {
            rig.push(1, 0);
        }
        rig.capture.is_grabbed() && !rig.inject.verified_inputs().is_empty()
    })
    .await;
    rig.inject.set_input_authorization(None);
    eventually("撤销后收回控制", || !rig.capture.is_grabbed()).await;
    assert!(rig.inject.events().is_empty());
}

#[tokio::test]
async fn ordinary_pairing_does_not_grant_another_devices_secure_input_authorization() {
    use crate::support::Setup;
    use ferry_platform_mock::MockInject;
    use ferry_trust::{Identity, InputSigner};
    let signer = InputSigner::new(&Identity::generate("other").unwrap()).unwrap();
    let inject = MockInject::new();
    inject.set_input_authorization(Some(signer.authorization().clone()));
    let rig = Rig::with(Setup {
        pc_inject: Some(inject),
        ..Setup::default()
    })
    .await;
    rig.enter().await;
    assert!(rig.inject.verified_inputs().is_empty());
    assert!(!rig.inject.events().is_empty(), "普通权限共享仍然可用");
}

#[tokio::test]
async fn secure_input_rejects_invalid_frames_and_survives_a_stalled_data_handshake() {
    use crate::support::{AutoConfirm, Device, display};
    use ferry_engine::{ClientConfig, EdgeFeed, Events, Files, ServerAddr, connect};
    use ferry_net::Tls;
    use ferry_platform_mock::{MockClipboard, MockDisplays, MockInject};
    use ferry_proto::{Capabilities, Enter, Message, SecureAction};
    use ferry_trust::{InputSigner, Peer};
    use ferry_types::Point;
    use tokio::net::TcpListener;

    // 0 未签署，1 篡改，2 重放，3 输入有效但数据连接故意不完成 TLS。
    for attack in 0..4 {
        let controller = Device::new();
        let receiver = Device::new();
        let identity = controller.store.identity("controller").unwrap();
        let signer = InputSigner::new(&identity).unwrap();
        let tls = Tls::new(&identity).unwrap();
        let mut peers = receiver.store.peers().unwrap();
        peers.add(Peer {
            name: "controller".into(),
            fingerprint: identity.fingerprint(),
            addr: None,
        });
        receiver.store.save_peers(&peers).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let inject = MockInject::new();
        inject.set_input_authorization(Some(signer.authorization().clone()));
        let config = ClientConfig {
            server: ServerAddr::Fixed(listener.local_addr().unwrap()),
            name: "receiver".into(),
            clipboard: attack == 3,
            clipboard_limit: Default::default(),
            command_key: false,
            store: receiver.store.clone(),
            events: Events::none(),
            files: Files::new(&receiver.store, Events::none()),
            edge: EdgeFeed::none(),
            devices: Vec::new(),
            commands: None,
            invite_port: None,
        };
        let client = tokio::spawn(connect(
            config,
            inject.clone(),
            MockDisplays::new(vec![display(1000, 800, 100)]),
            MockClipboard::new(),
            AutoConfirm::answering(false),
        ));
        let accepted = tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let (mut framed, _) = tls.accept(accepted.0).await.unwrap();
        ferry_net::accept(
            &mut framed,
            Capabilities::SECURE_INPUT
                | if attack == 3 {
                    Capabilities::CLIPBOARD
                } else {
                    Capabilities::empty()
                },
            "controller",
            |_| Ok(1),
        )
        .await
        .unwrap();
        assert!(matches!(
            framed.recv().await.unwrap(),
            Some(Message::Displays(_))
        ));
        let Some(Message::SecureChallenge { nonce }) = framed.recv().await.unwrap() else {
            panic!("缺少代理挑战");
        };
        let enter = Enter {
            seq: 1,
            position: Point::new(10, 20),
            locks: LockKeys::CAPS,
        };
        if attack == 0 {
            framed.send(&Message::Enter(enter)).await.unwrap();
        } else {
            let mut proof = signer
                .sign(nonce, 1, SecureAction::Enter(enter).encode().unwrap())
                .unwrap();
            if attack == 1 {
                proof.body[3] ^= 1;
            }
            framed.send(&Message::SecureInput(proof)).await.unwrap();
            if attack == 2 {
                framed.send(&Message::SecureInput(proof)).await.unwrap();
            }
        }
        if attack == 3 {
            let _stalled =
                tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            let until = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
            let mut nonce = nonce;
            let mut sequence = 2;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(until) => break,
                    message = framed.recv() => match message.unwrap() {
                        Some(Message::SecureChallenge { nonce: next }) => nonce = next,
                        Some(Message::Ping { nonce }) => framed.send(&Message::Pong { nonce }).await.unwrap(),
                        None | Some(Message::Bye(_)) => panic!("数据连接阻塞不应中断输入"),
                        _ => {},
                    },
                    _ = tick.tick() => {
                        let proof = signer.sign(nonce, sequence, SecureAction::Alive.encode().unwrap()).unwrap();
                        sequence += 1;
                        framed.send(&Message::SecureInput(proof)).await.unwrap();
                    }
                }
            }
            assert!(
                inject.verified_inputs().len() >= 5,
                "数据握手未完成时仍须处理输入并续期"
            );
            assert_eq!(inject.ended_input_sessions(), 0);
            assert!(inject.events().is_empty());
            client.abort();
            continue;
        }
        eventually("拒绝非法输入后关闭授权会话", || {
            inject.ended_input_sessions() > 0
        })
        .await;
        assert_eq!(inject.verified_inputs().len(), usize::from(attack == 2));
        assert!(inject.events().is_empty(), "非法报文不能退回未签署注入");
        client.abort();
    }
}

#[tokio::test]
async fn buttons_are_released_when_the_server_disappears() {
    let rig = Rig::start().await;
    rig.enter().await;
    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: true,
    });
    eventually("客户端收到按下", || rig.inject.events().len() >= 2).await;

    rig.server.abort();

    let released = InputEvent::Button {
        button: MouseButton::LEFT,
        pressed: false,
        position: ENTRY,
    };
    eventually("服务端消失后松开鼠标键", || {
        rig.inject.events().last() == Some(&released)
    })
    .await;
}

#[tokio::test]
async fn entering_syncs_caps_lock() {
    let rig = Rig::start().await;
    rig.capture.set_lock_keys(LockKeys::CAPS | LockKeys::NUM);
    rig.enter().await;

    eventually("客户端同步锁定键", || {
        !rig.inject.synced_locks().is_empty()
    })
    .await;
    assert_eq!(
        rig.inject.synced_locks(),
        [LockKeys::CAPS],
        "只同步 Caps Lock"
    );
}

#[tokio::test]
async fn keys_are_released_when_the_client_engine_is_stopped() {
    use ferry_types::{KeyAction, KeyCode};

    let rig = Rig::start().await;
    rig.enter().await;
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Down);
    let down = InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action: KeyAction::Down,
    };
    eventually("客户端按下 Shift", || {
        rig.inject.events().contains(&down)
    })
    .await;

    // 常驻后台暂停共享、重启引擎时，会直接丢掉客户端的任务。
    rig.client.abort();

    let up = InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action: KeyAction::Up,
    };
    eventually("引擎停下时松开 Shift", || {
        rig.inject.events().contains(&up)
    })
    .await;
}

#[tokio::test]
async fn the_local_cursor_is_hidden_while_the_pointer_is_on_the_server() {
    let rig = Rig::start().await;
    eventually(
        "连上后隐藏本机光标（光标在服务端）",
        || rig.inject.cursor_hidden() == [true],
    )
    .await;

    rig.enter().await;
    eventually("光标过来时显示", || {
        rig.inject.cursor_hidden() == [true, false]
    })
    .await;

    rig.leave().await;
    eventually("光标离开时再隐藏", || {
        rig.inject.cursor_hidden() == [true, false, true]
    })
    .await;

    // 服务端消失：会话结束，光标一定要恢复显示。
    rig.server.abort();
    eventually("断开后恢复显示", || {
        rig.inject.cursor_hidden() == [true, false, true, false]
    })
    .await;
}

#[tokio::test]
async fn an_injection_failure_returns_control_and_retries_a_failed_key_release() {
    use ferry_platform::Error;
    use ferry_types::{KeyAction, KeyCode};

    let rig = Rig::start().await;
    rig.enter().await;
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Down);
    let key = |action| InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action,
    };
    eventually("Shift 已按下", || {
        rig.inject.events().contains(&key(KeyAction::Down))
    })
    .await;
    rig.inject.fail_next(Error::Os {
        call: "SendInput",
        detail: "模拟失败".into(),
    });
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Up);
    eventually("注入失败后归还控制", || !rig.capture.is_grabbed()).await;
    eventually("松开失败的键仍然会补发释放", || {
        rig.inject.events().contains(&key(KeyAction::Up))
    })
    .await;
    assert!(!rig.client.is_finished(), "客户端应当自动重连");
    rig.enter().await;
}

#[tokio::test]
async fn a_desktop_change_without_new_input_returns_control_and_releases_after_recovery() {
    use ferry_types::{KeyAction, KeyCode};

    let rig = Rig::start().await;
    rig.enter().await;
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Down);
    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: true,
    });
    let key = |action| InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action,
    };
    let button = |pressed| InputEvent::Button {
        button: MouseButton::LEFT,
        pressed,
        position: ENTRY,
    };
    eventually("远端按着 Shift 和鼠标键", || {
        let events = rig.inject.events();
        events.contains(&key(KeyAction::Down)) && events.contains(&button(true))
    })
    .await;

    rig.inject.set_available(false);
    // 不再发送移动，模拟弹出 UAC 后用户等着看屏幕。
    eventually("轮询发现桌面不可用并归还控制", || {
        !rig.capture.is_grabbed()
    })
    .await;
    assert!(!rig.inject.events().contains(&key(KeyAction::Up)));
    assert!(!rig.inject.events().contains(&button(false)));
    rig.inject.set_available(true);
    eventually("桌面恢复后补发所有释放", || {
        let events = rig.inject.events();
        events.contains(&key(KeyAction::Up)) && events.contains(&button(false))
    })
    .await;
    // 控制端的实体鼠标也已松开，之后可以再次进入。
    rig.capture.emit(CaptureEvent::Button {
        button: MouseButton::LEFT,
        pressed: false,
    });
    // 整个过程连接没有断：断开重连会多出一次显示、一次隐藏光标。
    assert_eq!(rig.inject.cursor_hidden(), [true, false, true]);
    rig.enter().await;
}

#[tokio::test]
async fn an_unavailable_desktop_refuses_the_cursor_but_keeps_the_connection() {
    let rig = Rig::start().await;
    eventually("连上后隐藏本机光标", || {
        rig.inject.cursor_hidden() == [true]
    })
    .await;

    // 光标还在控制端时本机锁屏或切到管理员窗口：不影响连接。
    rig.inject.set_available(false);
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(!rig.client.is_finished());

    // 光标要过来：本机拒绝，控制端收回键鼠。
    rig.push(1, 0);
    eventually("被拒绝后控制端收回键鼠", || {
        !rig.capture.is_grabbed()
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!rig.capture.is_grabbed());
    assert!(rig.inject.events().is_empty(), "不可用期间不注入任何键鼠");

    // 恢复后照常进入；中间没有断开重连过。
    rig.inject.set_available(true);
    rig.enter().await;
    assert_eq!(rig.inject.cursor_hidden(), [true, false]);
}

#[tokio::test]
async fn unsupported_input_is_skipped_without_ending_the_session() {
    use ferry_platform::Error;
    use ferry_types::{KeyAction, KeyCode};

    let rig = Rig::start().await;
    rig.enter().await;
    rig.inject.fail_next(Error::Unsupported("模拟不支持的键"));
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Down);
    rig.key(KeyCode::LEFT_SHIFT, KeyAction::Up);
    let up = InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action: KeyAction::Up,
    };
    eventually("不支持的键不影响后续输入", || {
        rig.inject.events().contains(&up)
    })
    .await;
    assert!(rig.capture.is_grabbed());
}

#[tokio::test]
async fn a_failed_cursor_update_does_not_skip_restoration_when_stopped() {
    use ferry_platform::Error;

    let rig = Rig::start().await;
    rig.enter().await;
    rig.inject.fail_next_cursor_update(Error::Os {
        call: "SendMessageTimeoutW",
        detail: "模拟超时，但消息可能已经执行".into(),
    });
    rig.leave().await;
    eventually("尝试隐藏光标", || {
        rig.inject.cursor_hidden() == [true, false, true]
    })
    .await;
    rig.client.abort();
    eventually("未知状态仍要恢复光标", || {
        rig.inject.cursor_hidden() == [true, false, true, false]
    })
    .await;
}

// ── 按地址连接 ───────────────────────────────

mod by_address {
    use ferry_engine::{ClientCommand, DialError, Event};

    use crate::support::{Rig, Setup};

    async fn server_only() -> Rig {
        Rig::with(Setup {
            start_client: false,
            ..Setup::default()
        })
        .await
    }

    #[tokio::test]
    async fn a_client_connects_to_an_address_when_asked_and_pairs() {
        let rig = server_only().await;
        let mut pc = rig.idle_client("书房", false);
        pc.commands.send(ClientCommand::Connect(rig.addr)).unwrap();

        let error = pc
            .wait_for("按地址连接的结果", |e| match e {
                Event::Dialed { error, invite, .. } if !invite => Some(*error),
                _ => None,
            })
            .await;
        assert_eq!(error, None, "连上了，开始核对配对码");
        // 配对完立即用这个地址重连，不用等局域网发现。
        pc.wait_for("配对后连上服务端", |e| {
            matches!(e, Event::Connected { .. }).then_some(())
        })
        .await;
        let peers = pc.device.store.peers().unwrap();
        let server = peers.iter().next().expect("记住了服务端");
        assert_eq!(server.addr, Some(rig.addr), "记下了服务端的地址");
    }

    #[tokio::test]
    async fn an_address_where_nothing_waits_is_reported() {
        let rig = server_only().await;
        let closed = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap()
        };
        let mut pc = rig.idle_client("书房", false);
        pc.commands.send(ClientCommand::Connect(closed)).unwrap();
        let error = pc
            .wait_for("按地址连接的结果", |e| match e {
                Event::Dialed { error, .. } => Some(*error),
                _ => None,
            })
            .await;
        assert_eq!(error, Some(DialError::Refused));
    }
}
