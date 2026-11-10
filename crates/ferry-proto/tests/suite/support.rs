//! 测试共用：每种消息的样例、编码帮助函数、可复现的伪随机数。

use ferry_proto::{
    Bye, ByeReason, Cancel, CancelReason, Capabilities, Channel, Chunk, ClipboardItem,
    ClipboardOffer, ClipboardRequest, Codec, DropHint, Enter, EntryKind, FileCancel, FileEnd,
    FileEntry, FileList, FileOffer, FileResume, Hello, HintState, Message, Reject, RejectReason,
    WIRE_FORMAT, Welcome,
};
use ferry_types::{
    ClipboardFormat, Display, DisplayId, InputEvent, KeyAction, KeyCode, LockKeys, MouseButton,
    Point, Rect, Side, TransferId, WHEEL_NOTCH,
};

/// 样例里用的传输号。
pub const TRANSFER: TransferId = TransferId([0xA5; 16]);

/// 每种消息至少一个样例，以及它所属的连接。
pub fn samples() -> Vec<(Channel, Message)> {
    let at = Point::new(-120, 340);
    let input = |event| (Channel::Input, Message::Input(event));

    vec![
        (Channel::Input, Message::SecureChallenge { nonce: [7; 32] }),
        (
            Channel::Input,
            Message::SecureInput(ferry_types::InputProof {
                challenge: [8; 32],
                sequence: 1,
                body: [0; 32],
                signature: [9; 64],
            }),
        ),
        (
            Channel::Input,
            Message::Hello(Hello::new(
                Channel::Input,
                Capabilities::CLIPBOARD,
                0,
                "书房的 Mac",
            )),
        ),
        (
            Channel::Data,
            Message::Welcome(Welcome {
                format: WIRE_FORMAT,
                capabilities: Capabilities::all(),
                session: u64::MAX,
                name: "办公室 PC".into(),
            }),
        ),
        (
            Channel::Input,
            Message::Reject(Reject::new(RejectReason::BUSY)),
        ),
        (
            Channel::Data,
            Message::Bye(Bye {
                reason: ByeReason::TIMEOUT,
            }),
        ),
        (
            Channel::Input,
            Message::Ping {
                nonce: 0x0123_4567_89AB_CDEF,
            },
        ),
        (Channel::Data, Message::Pong { nonce: 7 }),
        (
            Channel::Pair,
            Message::PairCommit {
                commitment: [0xC0; 32],
            },
        ),
        (Channel::Pair, Message::PairNonce { nonce: [7; 32] }),
        (Channel::Pair, Message::PairResult { accepted: true }),
        (
            Channel::Input,
            Message::Displays(vec![
                Display {
                    id: DisplayId(1),
                    bounds: Rect::new(0, 0, 2560, 1440),
                    scale_percent: 200,
                    primary: true,
                },
                Display {
                    id: DisplayId(2),
                    bounds: Rect::new(-1920, 0, 1920, 1080),
                    scale_percent: 100,
                    primary: false,
                },
            ]),
        ),
        (
            Channel::Input,
            Message::Enter(Enter {
                seq: 42,
                position: at,
                locks: LockKeys::CAPS | LockKeys::NUM,
            }),
        ),
        (Channel::Input, Message::Leave { seq: 42 }),
        (Channel::Input, Message::PeerSide { side: Side::Right }),
        (Channel::Input, Message::OpenSettings),
        (Channel::Input, Message::InputBlocked),
        (
            Channel::Input,
            Message::AppInfo {
                version: "0.2.0".into(),
            },
        ),
        (
            Channel::Input,
            Message::DropHint(DropHint {
                state: HintState::Over,
                along: 4_800,
                closeness: 100,
                name: "证书.p12".into(),
                items: 1,
                bytes: 3_277,
            }),
        ),
        input(InputEvent::Key {
            code: KeyCode::LEFT_SHIFT,
            action: KeyAction::Down,
        }),
        input(InputEvent::Key {
            code: KeyCode::new(KeyCode::CONSUMER_PAGE, 0xE9),
            action: KeyAction::Repeat,
        }),
        input(InputEvent::PointerMotion { position: at }),
        input(InputEvent::PointerDelta { dx: -3, dy: 5 }),
        input(InputEvent::Button {
            button: MouseButton::RIGHT,
            pressed: true,
            position: at,
        }),
        input(InputEvent::Wheel {
            dx: 0,
            dy: -WHEEL_NOTCH,
            position: at,
        }),
        (
            Channel::Data,
            Message::ClipboardOffer(ClipboardOffer {
                seq: 9,
                items: vec![
                    ClipboardItem {
                        format: ClipboardFormat::TEXT,
                        size: 12,
                    },
                    ClipboardItem {
                        format: ClipboardFormat::PNG,
                        size: 1 << 20,
                    },
                ],
            }),
        ),
        (
            Channel::Data,
            Message::ClipboardRequest(ClipboardRequest {
                seq: 9,
                format: ClipboardFormat::PNG,
                transfer: 3,
            }),
        ),
        (
            Channel::Data,
            Message::Chunk(Chunk {
                transfer: 3,
                last: true,
                data: b"hello".to_vec(),
            }),
        ),
        (
            Channel::Data,
            Message::Cancel(Cancel {
                transfer: 3,
                reason: CancelReason::STALE,
            }),
        ),
        (
            Channel::Data,
            Message::FileOffer(FileOffer {
                id: TRANSFER,
                name: "设计稿".into(),
                items: 2,
                files: 37,
                bytes: 88_080_384,
            }),
        ),
        (Channel::Data, Message::FileAccept { id: TRANSFER }),
        (
            Channel::Data,
            Message::FileCancel(FileCancel {
                id: TRANSFER,
                reason: CancelReason::CANCELLED,
            }),
        ),
        (Channel::File, Message::FileOpen { id: TRANSFER }),
        (
            Channel::File,
            Message::FileList(FileList {
                entries: vec![
                    FileEntry {
                        path: "设计稿".into(),
                        kind: EntryKind::Dir,
                        size: 0,
                        modified: 0,
                    },
                    FileEntry {
                        path: "设计稿/首页.png".into(),
                        kind: EntryKind::File,
                        size: 2_400_000,
                        modified: 1_793_400_000_000,
                    },
                ],
                last: true,
            }),
        ),
        (
            Channel::File,
            Message::FileResume(FileResume {
                index: 1,
                offset: 65_536,
            }),
        ),
        (
            Channel::File,
            Message::FileData {
                data: vec![0xEE; 300],
            },
        ),
        (
            Channel::File,
            Message::FileEnd(FileEnd {
                index: 1,
                sha256: [0x5A; 32],
            }),
        ),
        (Channel::File, Message::FileDone),
    ]
}

pub fn encode(channel: Channel, message: &Message) -> Vec<u8> {
    let mut buf = Vec::new();
    Codec::new(channel)
        .encode(message, &mut buf)
        .expect("样例消息必须能编码");
    buf
}

/// 手工拼一帧，用于构造非法数据。
pub fn frame(ty: u16, payload: &[u8]) -> Vec<u8> {
    let mut buf = ty.to_le_bytes().to_vec();
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload);
    buf
}

/// xorshift64 伪随机数：种子固定，失败可以稳定复现。
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}
