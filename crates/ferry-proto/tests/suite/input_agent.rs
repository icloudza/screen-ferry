use ferry_proto::input_agent::{Command, FRAME_LEN, Request, Response, Status};
use ferry_types::{InputEvent, KeyAction, KeyCode, LockKeys, MouseButton, Point};

fn request(command: Command) -> Request {
    Request {
        sequence: 1,
        expires_ms: 400,
        command,
    }
}

#[test]
fn only_the_narrow_command_set_round_trips_in_fixed_frames() {
    let point = Point::new(-1920, 900);
    for command in [
        Command::Check,
        Command::Bind { peer: [3; 32] },
        Command::Verified(ferry_types::InputProof {
            challenge: [4; 32],
            sequence: 1,
            body: [0; 32],
            signature: [5; 64],
        }),
        Command::Release,
        Command::Cursor { hidden: true },
        Command::Locks {
            wanted: LockKeys::CAPS,
            mask: LockKeys::CAPS | LockKeys::NUM,
        },
        Command::Input(InputEvent::Key {
            code: KeyCode::LEFT_SHIFT,
            action: KeyAction::Up,
        }),
        Command::Input(InputEvent::PointerMotion { position: point }),
        Command::Input(InputEvent::PointerDelta { dx: -900, dy: 400 }),
        Command::Input(InputEvent::Button {
            button: MouseButton::LEFT,
            pressed: false,
            position: point,
        }),
        Command::Input(InputEvent::Wheel {
            dx: 120,
            dy: -60,
            position: point,
        }),
    ] {
        let expected = request(command);
        let frame = expected.encode().unwrap();
        assert_eq!(frame.len(), FRAME_LEN);
        assert_eq!(Request::decode(&frame).unwrap(), expected);
    }
}

#[test]
fn rejects_wrong_versions_sizes_opcodes_and_nonzero_padding() {
    let valid = request(Command::Check).encode().unwrap();
    for len in 0..FRAME_LEN {
        assert!(Request::decode(&valid[..len]).is_err());
    }
    assert!(Request::decode(&[0; FRAME_LEN + 1]).is_err());
    for (index, value) in [(0, b'X'), (4, 3), (6, 255), (7, 1), (24, 99), (63, 1)] {
        let mut invalid = valid;
        invalid[index] = value;
        assert!(Request::decode(&invalid).is_err(), "byte {index}");
    }
    let mut extra = valid;
    extra[6] = 1;
    assert!(Request::decode(&extra).is_err(), "零填充也不能冒充命令字段");
}

#[test]
fn rejects_non_input_network_messages_and_invalid_event_fields() {
    let mut frame = request(Command::Input(InputEvent::Key {
        code: KeyCode::LEFT_SHIFT,
        action: KeyAction::Down,
    }))
    .encode()
    .unwrap();
    frame[25..27].copy_from_slice(&1_u16.to_le_bytes());
    assert!(
        Request::decode(&frame).is_err(),
        "不能借嵌套消息调用握手或其他网络功能"
    );
    let mut cursor = request(Command::Cursor { hidden: false }).encode().unwrap();
    cursor[25] = 2;
    assert!(Request::decode(&cursor).is_err());
    let mut button = request(Command::Input(InputEvent::Button {
        button: MouseButton::LEFT,
        pressed: false,
        position: Point::default(),
    }))
    .encode()
    .unwrap();
    button[27] = 0;
    assert!(Request::decode(&button).is_err());
}

#[test]
fn replies_are_typed_and_cannot_be_used_as_requests() {
    for status in [
        Status::Ok,
        Status::Unsupported,
        Status::Unavailable,
        Status::Expired,
        Status::Failed,
        Status::Unauthorized,
    ] {
        let reply = Response {
            sequence: 51,
            status,
            challenge: None,
        };
        let bytes = reply.encode();
        assert_eq!(Response::decode(&bytes).unwrap(), reply);
        assert!(Request::decode(&bytes).is_err());
    }
    let response = Response {
        sequence: 1,
        status: Status::Ok,
        challenge: Some([7; 32]),
    };
    assert_eq!(Response::decode(&response.encode()).unwrap(), response);
    let mut invalid = response.encode();
    invalid[25] = 5;
    assert!(Response::decode(&invalid).is_err(), "未授权回复不能带挑战");
    let mut reply = Response {
        sequence: 1,
        status: Status::Ok,
        challenge: None,
    }
    .encode();
    reply[25] = 255;
    assert!(Response::decode(&reply).is_err());
    reply[25] = 0;
    reply[16] = 1;
    assert!(
        Response::decode(&reply).is_err(),
        "保留字段不能承载隐藏数据"
    );
}

#[test]
fn every_single_bit_mutation_is_rejected_or_has_one_canonical_encoding() {
    for command in [
        Command::Check,
        Command::Cursor { hidden: true },
        Command::Input(InputEvent::Wheel {
            dx: 120,
            dy: -120,
            position: Point::new(-1, 800),
        }),
    ] {
        let original = request(command).encode().unwrap();
        for byte in 0..FRAME_LEN {
            for bit in 0..8 {
                let mut mutated = original;
                mutated[byte] ^= 1 << bit;
                if let Ok(decoded) = Request::decode(&mutated) {
                    assert_eq!(decoded.encode().unwrap(), mutated);
                }
            }
        }
    }
}
