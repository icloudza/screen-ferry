use std::collections::BTreeSet;

use ferry_proto::limits::{MAX_CHUNK_LEN, MAX_DISPLAYS, MAX_NAME_LEN};
use ferry_proto::{Capabilities, Channel, Chunk, Codec, DecodeError, EncodeError, Hello, Message};
use ferry_types::{Display, DisplayId, InputEvent, KeyAction, KeyCode, Point, Rect};

use crate::support::{encode, frame, samples};

/// 新增消息时这里会编译失败，提醒同时补充样例和下面的数量。
fn kind(message: &Message) -> &'static str {
    match message {
        Message::Hello(_) => "Hello",
        Message::Welcome(_) => "Welcome",
        Message::Reject(_) => "Reject",
        Message::Bye(_) => "Bye",
        Message::Ping { .. } => "Ping",
        Message::Pong { .. } => "Pong",
        Message::PairCommit { .. } => "PairCommit",
        Message::PairNonce { .. } => "PairNonce",
        Message::PairResult { .. } => "PairResult",
        Message::Displays(_) => "Displays",
        Message::Enter(_) => "Enter",
        Message::Leave { .. } => "Leave",
        Message::PeerSide { .. } => "PeerSide",
        Message::DropHint(_) => "DropHint",
        Message::OpenSettings => "OpenSettings",
        Message::AppInfo { .. } => "AppInfo",
        Message::Input(event) => match event {
            InputEvent::Key { .. } => "Key",
            InputEvent::PointerMotion { .. } => "PointerMotion",
            InputEvent::PointerDelta { .. } => "PointerDelta",
            InputEvent::Button { .. } => "Button",
            InputEvent::Wheel { .. } => "Wheel",
        },
        Message::SecureChallenge { .. } => "SecureChallenge",
        Message::SecureInput(_) => "SecureInput",
        Message::ClipboardOffer(_) => "ClipboardOffer",
        Message::ClipboardRequest(_) => "ClipboardRequest",
        Message::Chunk(_) => "Chunk",
        Message::Cancel(_) => "Cancel",
        Message::FileOffer(_) => "FileOffer",
        Message::FileAccept { .. } => "FileAccept",
        Message::FileCancel(_) => "FileCancel",
        Message::FileOpen { .. } => "FileOpen",
        Message::FileList(_) => "FileList",
        Message::FileResume(_) => "FileResume",
        Message::FileData { .. } => "FileData",
        Message::FileEnd(_) => "FileEnd",
        Message::FileDone => "FileDone",
    }
}

const MESSAGE_KINDS: usize = 36;

fn decode(channel: Channel, buf: &[u8]) -> Result<Option<(Message, usize)>, DecodeError> {
    Codec::new(channel).decode(buf)
}

#[test]
fn samples_cover_every_message() {
    let covered: BTreeSet<_> = samples().iter().map(|(_, m)| kind(m)).collect();
    assert_eq!(covered.len(), MESSAGE_KINDS);
}

#[test]
fn every_message_roundtrips() {
    for (channel, message) in samples() {
        let buf = encode(channel, &message);
        let (decoded, used) = decode(channel, &buf).unwrap().expect("应解出完整一帧");
        assert_eq!(used, buf.len(), "{message:?}");
        assert_eq!(decoded, message);
    }
}

/// 线上格式一旦发布就不能改。这里的字节改变，说明协议被无意中改了。
#[test]
fn wire_format_is_frozen() {
    let key_a = Message::Input(InputEvent::Key {
        code: KeyCode::keyboard(0x04),
        action: KeyAction::Down,
    });
    assert_eq!(
        encode(Channel::Input, &key_a),
        [0x10, 0x01, 5, 0, 0, 0, 0x04, 0x00, 0x07, 0x00, 1],
    );

    let motion = Message::Input(InputEvent::PointerMotion {
        position: Point::new(-1, 2),
    });
    assert_eq!(
        encode(Channel::Input, &motion),
        [0x20, 0x01, 8, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 2, 0, 0, 0],
    );

    let hello = Message::Hello(Hello::new(Channel::Input, Capabilities::CLIPBOARD, 0, "A"));
    #[rustfmt::skip]
    let expected = [
        0x01, 0x00, 22, 0, 0, 0,  // 类型 Hello，消息体 22 字节
        2, 0,                     // 版本 2：签署输入协议
        1,                        // 输入连接
        2, 0, 0, 0, 0, 0, 0, 0,   // 能力：CLIPBOARD
        0, 0, 0, 0, 0, 0, 0, 0,   // 会话号 0
        1, 0, b'A',               // 设备名 "A"
    ];
    assert_eq!(encode(Channel::Input, &hello), expected);
}

#[test]
fn rejects_invalid_values() {
    let origin = [0u8; 8];
    let cases: [(u16, Vec<u8>, &str); 6] = [
        (0x0122, [&[0, 1][..], &origin].concat(), "鼠标按键编号"),
        (0x0122, [&[1, 2][..], &origin].concat(), "布尔值"),
        (0x0110, vec![0x04, 0x00, 0x07, 0x00, 9], "按键动作"),
        (
            0x0101,
            [&[0; 4][..], &origin, &[0x80]].concat(),
            "锁定键状态",
        ),
        (
            0x0001,
            [&[1, 0, 9][..], &[0; 16], &[0, 0]].concat(),
            "连接类型",
        ),
        (
            0x0001,
            [&[1, 0, 1][..], &[0; 16], &[1, 0, 0xFF]].concat(),
            "UTF-8 字符串",
        ),
    ];
    for (ty, payload, what) in cases {
        assert_eq!(
            decode(Channel::Input, &frame(ty, &payload)),
            Err(DecodeError::InvalidValue(what)),
        );
    }
}

#[test]
fn body_must_be_consumed_exactly() {
    assert_eq!(
        decode(Channel::Input, &frame(0x0005, &[0; 9])),
        Err(DecodeError::TrailingBytes)
    );
    assert_eq!(
        decode(Channel::Input, &frame(0x0005, &[0; 7])),
        Err(DecodeError::Truncated)
    );
}

#[test]
fn enforces_name_limit_both_ways() {
    let long = "名".repeat(MAX_NAME_LEN / 3 + 1);
    let hello = Message::Hello(Hello::new(Channel::Input, Capabilities::empty(), 0, long));
    assert_eq!(
        Codec::new(Channel::Input).encode(&hello, &mut Vec::new()),
        Err(EncodeError::LimitExceeded("设备名长度")),
    );

    let len = (MAX_NAME_LEN as u16 + 1).to_le_bytes();
    let payload = [&[1, 0, 1][..], &[0; 16], &len, &[b'x'; MAX_NAME_LEN + 1]].concat();
    assert_eq!(
        decode(Channel::Input, &frame(0x0001, &payload)),
        Err(DecodeError::LimitExceeded("设备名长度")),
    );
}

#[test]
fn enforces_display_limit() {
    let display = Display {
        id: DisplayId(0),
        bounds: Rect::new(0, 0, 1, 1),
        scale_percent: 100,
        primary: false,
    };
    let codec = Codec::new(Channel::Input);
    let mut buf = Vec::new();

    assert!(
        codec
            .encode(&Message::Displays(vec![display; MAX_DISPLAYS]), &mut buf)
            .is_ok()
    );
    assert_eq!(
        codec.encode(
            &Message::Displays(vec![display; MAX_DISPLAYS + 1]),
            &mut buf
        ),
        Err(EncodeError::LimitExceeded("显示器数量")),
    );
}

#[test]
fn enforces_chunk_limit() {
    let chunk = |len| {
        Message::Chunk(Chunk {
            transfer: 1,
            last: false,
            data: vec![0; len],
        })
    };
    let codec = Codec::new(Channel::Data);
    let mut buf = Vec::new();

    assert!(codec.encode(&chunk(MAX_CHUNK_LEN), &mut buf).is_ok());
    assert_eq!(
        codec.encode(&chunk(MAX_CHUNK_LEN + 1), &mut buf),
        Err(EncodeError::LimitExceeded("分块大小")),
    );
}

#[test]
fn file_messages_wire_format_is_frozen() {
    use ferry_proto::{FileOffer, FileResume};
    use ferry_types::TransferId;

    let resume = Message::FileResume(FileResume {
        index: 2,
        offset: 1,
    });
    #[rustfmt::skip]
    let expected = [
        0x12, 0x04, 12, 0, 0, 0,   // 类型 FileResume，消息体 12 字节
        2, 0, 0, 0,                // 第 2 个条目
        1, 0, 0, 0, 0, 0, 0, 0,    // 第 1 个字节
    ];
    assert_eq!(encode(Channel::File, &resume), expected);

    let offer = Message::FileOffer(FileOffer {
        id: TransferId([1; 16]),
        name: "a".into(),
        items: 1,
        files: 1,
        bytes: 5,
    });
    let bytes = encode(Channel::Data, &offer);
    assert_eq!(&bytes[..2], &[0x00, 0x04], "类型 FileOffer");
    assert_eq!(bytes.len(), 6 + 16 + 3 + 4 + 4 + 8);
}

#[test]
fn file_lists_and_paths_are_bounded() {
    use ferry_proto::limits::{MAX_FILE_LIST, MAX_PATH_LEN};
    use ferry_proto::{EntryKind, FileEntry, FileList};

    let entry = |path: String| FileEntry {
        path,
        kind: EntryKind::File,
        size: 1,
        modified: 0,
    };
    let full = Message::FileList(FileList {
        entries: (0..MAX_FILE_LIST)
            .map(|_| entry("x".repeat(MAX_PATH_LEN)))
            .collect(),
        last: false,
    });
    let mut buf = Vec::new();
    Codec::new(Channel::File)
        .encode(&full, &mut buf)
        .expect("最满的一段清单也放得进一帧");

    let too_many = Message::FileList(FileList {
        entries: (0..=MAX_FILE_LIST).map(|_| entry("x".into())).collect(),
        last: true,
    });
    assert!(
        Codec::new(Channel::File)
            .encode(&too_many, &mut Vec::new())
            .is_err()
    );

    let too_long = Message::FileList(FileList {
        entries: vec![entry("x".repeat(MAX_PATH_LEN + 1))],
        last: true,
    });
    assert!(
        Codec::new(Channel::File)
            .encode(&too_long, &mut Vec::new())
            .is_err()
    );
}

#[test]
fn drop_hints_keep_their_ranges() {
    use ferry_proto::{DropHint, HintState};

    let hint = |along, closeness| {
        Message::DropHint(DropHint {
            state: HintState::Near,
            along,
            closeness,
            name: String::new(),
            items: 1,
            bytes: 0,
        })
    };
    let mut buf = Vec::new();
    assert!(
        Codec::new(Channel::Input)
            .encode(&hint(10_000, 100), &mut buf)
            .is_ok()
    );
    assert!(
        Codec::new(Channel::Input)
            .encode(&hint(10_001, 0), &mut Vec::new())
            .is_err()
    );
    assert!(
        Codec::new(Channel::Input)
            .encode(&hint(0, 101), &mut Vec::new())
            .is_err()
    );

    // 解码时同样检查：手工把 along 改成 10001。
    let along_at = 6 + 1;
    buf[along_at..along_at + 2].copy_from_slice(&10_001u16.to_le_bytes());
    assert!(Codec::new(Channel::Input).decode(&buf).is_err());
}
