//! 消息总表：类型号、所属连接、消息体分发。

use ferry_types::{
    Display, InputEvent, InputProof, KeyAction, KeyCode, MouseButton, Point, Side, TransferId,
};

use crate::clipboard::{ClipboardOffer, ClipboardRequest};
use crate::edge::DropHint;
use crate::error::{DecodeError, EncodeError};
use crate::file::{self, FileCancel, FileEnd, FileList, FileOffer, FileResume};
use crate::handshake::{Bye, Channel, Hello, Reject, Welcome};
use crate::limits::{MAX_DISPLAYS, MAX_VERSION_LEN, PAIRING_NONCE_LEN};
use crate::screen::Enter;
use crate::transfer::{Cancel, Chunk};
use crate::wire::{Body, Reader, Writer};

/// 协议中的一条消息。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    // ── 两种连接通用 ──────────────────────
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    Bye(Bye),
    /// 心跳。对方须尽快以同一个 `nonce` 回复 `Pong`，发送方据此计算往返时间。
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },

    // ── 配对连接 ──────────────────────────
    /// 接受方对自己配对随机数的承诺：`SHA-256(随机数)`。先承诺、后公开，
    /// 中间人就无法看到发起方的随机数后再挑选自己的随机数去凑配对码。
    PairCommit {
        commitment: [u8; PAIRING_NONCE_LEN],
    },
    /// 配对随机数。发起方收到承诺后先发，接受方收到后再公开自己的。
    PairNonce {
        nonce: [u8; PAIRING_NONCE_LEN],
    },
    /// 本机用户是否确认了配对码。双方都确认才算配对成功。
    PairResult {
        accepted: bool,
    },

    // ── 输入连接 ──────────────────────────
    /// 发送方的显示器布局。连接建立和布局变化时发送。
    Displays(Vec<Display>),
    Enter(Enter),
    /// 光标离开接收方的屏幕。接收方应释放所有仍按下的键和鼠标按键。
    Leave {
        seq: u32,
    },
    Input(InputEvent),
    SecureChallenge {
        nonce: [u8; 32],
    },
    SecureInput(InputProof),
    /// 发送方位于接收方的哪一侧。会话开始和方向改变时发送。需要 `EDGE_DROP` 能力。
    PeerSide {
        side: Side,
    },
    /// 拖动中的提示，见 [`DropHint`]。需要 `EDGE_DROP` 能力。
    DropHint(DropHint),
    /// 请接收方打开它的设置界面，翻到发送方这台设备的设置页。需要 `REMOTE_SETTINGS` 能力。
    /// 被控制的一方想改“对方控制本机时”的滚动、按键等设置时用：这些设置在控制的一方。
    OpenSettings,
    /// 被控端暂时收不了键鼠（锁屏、权限提示、前台是管理员窗口或注入失败），请控制端把光标收回去。
    /// 连接保持，剪贴板和文件照常。需要 `INPUT_BLOCKED` 能力。
    InputBlocked,
    /// 发送方的程序版本（例如 `0.2.0`），握手后发一次。需要 `APP_INFO` 能力。
    /// 设置界面据此显示对方的版本，提醒两台一起更新。
    AppInfo {
        version: String,
    },

    // ── 数据连接 ──────────────────────────
    ClipboardOffer(ClipboardOffer),
    ClipboardRequest(ClipboardRequest),
    Chunk(Chunk),
    Cancel(Cancel),
    /// 提议发送一批文件。需要 `FILES` 能力。
    FileOffer(FileOffer),
    /// 接收方同意接收，随后由客户端打开文件连接。
    FileAccept {
        id: TransferId,
    },
    FileCancel(FileCancel),

    // ── 文件连接 ──────────────────────────
    /// 客户端打开文件连接后的第一条消息：这条连接用于哪次传输。
    FileOpen {
        id: TransferId,
    },
    FileList(FileList),
    FileResume(FileResume),
    /// 当前文件的下一段内容。
    FileData {
        data: Vec<u8>,
    },
    FileEnd(FileEnd),
    /// 全部收好、校验通过并已放到最终位置。
    FileDone,
}

/// 消息类型号。高字节是类别：0x00 握手、心跳与配对，0x01 输入，0x02 剪贴板，0x03 传输，0x04 文件。
mod ty {
    pub(super) const HELLO: u16 = 0x0001;
    pub(super) const WELCOME: u16 = 0x0002;
    pub(super) const REJECT: u16 = 0x0003;
    pub(super) const BYE: u16 = 0x0004;
    pub(super) const PING: u16 = 0x0005;
    pub(super) const PONG: u16 = 0x0006;
    pub(super) const PAIR_COMMIT: u16 = 0x0007;
    pub(super) const PAIR_NONCE: u16 = 0x0008;
    pub(super) const PAIR_RESULT: u16 = 0x0009;

    pub(super) const DISPLAYS: u16 = 0x0100;
    pub(super) const ENTER: u16 = 0x0101;
    pub(super) const LEAVE: u16 = 0x0102;
    pub(super) const PEER_SIDE: u16 = 0x0103;
    pub(super) const DROP_HINT: u16 = 0x0104;
    pub(super) const OPEN_SETTINGS: u16 = 0x0105;
    pub(super) const APP_INFO: u16 = 0x0106;
    pub(super) const SECURE_CHALLENGE: u16 = 0x0107;
    pub(super) const SECURE_INPUT: u16 = 0x0108;
    pub(super) const INPUT_BLOCKED: u16 = 0x0109;
    pub(super) const KEY: u16 = 0x0110;
    pub(super) const POINTER_MOTION: u16 = 0x0120;
    pub(super) const POINTER_DELTA: u16 = 0x0121;
    pub(super) const BUTTON: u16 = 0x0122;
    pub(super) const WHEEL: u16 = 0x0123;

    pub(super) const CLIPBOARD_OFFER: u16 = 0x0200;
    pub(super) const CLIPBOARD_REQUEST: u16 = 0x0201;

    pub(super) const CHUNK: u16 = 0x0300;
    pub(super) const CANCEL: u16 = 0x0301;

    pub(super) const FILE_OFFER: u16 = 0x0400;
    pub(super) const FILE_ACCEPT: u16 = 0x0401;
    pub(super) const FILE_CANCEL: u16 = 0x0402;
    pub(super) const FILE_OPEN: u16 = 0x0410;
    pub(super) const FILE_LIST: u16 = 0x0411;
    pub(super) const FILE_RESUME: u16 = 0x0412;
    pub(super) const FILE_DATA: u16 = 0x0413;
    pub(super) const FILE_END: u16 = 0x0414;
    pub(super) const FILE_DONE: u16 = 0x0415;
}

/// 类型号为 `ty` 的消息能否出现在 `channel` 上。未知类型返回 `None`。
pub(crate) fn allowed(ty: u16, channel: Channel) -> Option<bool> {
    let home = match ty {
        ty::HELLO | ty::WELCOME | ty::REJECT | ty::BYE | ty::PING | ty::PONG => return Some(true),
        ty::PAIR_COMMIT | ty::PAIR_NONCE | ty::PAIR_RESULT => Channel::Pair,
        ty::DISPLAYS
        | ty::ENTER
        | ty::LEAVE
        | ty::PEER_SIDE
        | ty::DROP_HINT
        | ty::OPEN_SETTINGS
        | ty::APP_INFO
        | ty::SECURE_CHALLENGE
        | ty::SECURE_INPUT
        | ty::INPUT_BLOCKED
        | ty::KEY
        | ty::POINTER_MOTION
        | ty::POINTER_DELTA
        | ty::BUTTON
        | ty::WHEEL => Channel::Input,
        ty::CLIPBOARD_OFFER
        | ty::CLIPBOARD_REQUEST
        | ty::CHUNK
        | ty::CANCEL
        | ty::FILE_OFFER
        | ty::FILE_ACCEPT
        | ty::FILE_CANCEL => Channel::Data,
        ty::FILE_OPEN
        | ty::FILE_LIST
        | ty::FILE_RESUME
        | ty::FILE_DATA
        | ty::FILE_END
        | ty::FILE_DONE => Channel::File,
        _ => return None,
    };
    Some(home == channel)
}

impl Message {
    pub(crate) fn is_input_type(kind: u16) -> bool {
        matches!(
            kind,
            ty::KEY | ty::POINTER_MOTION | ty::POINTER_DELTA | ty::BUTTON | ty::WHEEL
        )
    }
    /// 消息的类型号（见 docs/protocol.md 第 5 节）。日志里只记它，不记消息内容。
    pub fn type_id(&self) -> u16 {
        match self {
            Self::Hello(_) => ty::HELLO,
            Self::Welcome(_) => ty::WELCOME,
            Self::Reject(_) => ty::REJECT,
            Self::Bye(_) => ty::BYE,
            Self::Ping { .. } => ty::PING,
            Self::Pong { .. } => ty::PONG,
            Self::PairCommit { .. } => ty::PAIR_COMMIT,
            Self::PairNonce { .. } => ty::PAIR_NONCE,
            Self::PairResult { .. } => ty::PAIR_RESULT,
            Self::SecureChallenge { .. } => ty::SECURE_CHALLENGE,
            Self::SecureInput(_) => ty::SECURE_INPUT,
            Self::Displays(_) => ty::DISPLAYS,
            Self::Enter(_) => ty::ENTER,
            Self::Leave { .. } => ty::LEAVE,
            Self::PeerSide { .. } => ty::PEER_SIDE,
            Self::DropHint(_) => ty::DROP_HINT,
            Self::OpenSettings => ty::OPEN_SETTINGS,
            Self::InputBlocked => ty::INPUT_BLOCKED,
            Self::AppInfo { .. } => ty::APP_INFO,
            Self::Input(event) => match event {
                InputEvent::Key { .. } => ty::KEY,
                InputEvent::PointerMotion { .. } => ty::POINTER_MOTION,
                InputEvent::PointerDelta { .. } => ty::POINTER_DELTA,
                InputEvent::Button { .. } => ty::BUTTON,
                InputEvent::Wheel { .. } => ty::WHEEL,
            },
            Self::ClipboardOffer(_) => ty::CLIPBOARD_OFFER,
            Self::ClipboardRequest(_) => ty::CLIPBOARD_REQUEST,
            Self::Chunk(_) => ty::CHUNK,
            Self::Cancel(_) => ty::CANCEL,
            Self::FileOffer(_) => ty::FILE_OFFER,
            Self::FileAccept { .. } => ty::FILE_ACCEPT,
            Self::FileCancel(_) => ty::FILE_CANCEL,
            Self::FileOpen { .. } => ty::FILE_OPEN,
            Self::FileList(_) => ty::FILE_LIST,
            Self::FileResume(_) => ty::FILE_RESUME,
            Self::FileData { .. } => ty::FILE_DATA,
            Self::FileEnd(_) => ty::FILE_END,
            Self::FileDone => ty::FILE_DONE,
        }
    }

    pub(crate) fn encode_body(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        match self {
            Self::SecureChallenge { nonce } => {
                w.array(nonce);
                Ok(())
            }
            Self::SecureInput(proof) => proof.encode(w),
            Self::Hello(m) => m.encode(w),
            Self::Welcome(m) => m.encode(w),
            Self::Reject(m) => m.encode(w),
            Self::Bye(m) => m.encode(w),
            Self::Ping { nonce } | Self::Pong { nonce } => {
                w.u64(*nonce);
                Ok(())
            }
            Self::PairCommit { commitment: bytes } | Self::PairNonce { nonce: bytes } => {
                w.array(bytes);
                Ok(())
            }
            Self::PairResult { accepted } => {
                w.bool(*accepted);
                Ok(())
            }
            Self::Displays(displays) => w.list(displays, MAX_DISPLAYS, "显示器数量"),
            Self::Enter(m) => m.encode(w),
            Self::Leave { seq } => {
                w.u32(*seq);
                Ok(())
            }
            Self::Input(event) => encode_input(event, w),
            Self::PeerSide { side } => side.encode(w),
            Self::DropHint(m) => m.encode(w),
            Self::AppInfo { version } => w.str(version, MAX_VERSION_LEN, "版本号长度"),
            Self::ClipboardOffer(m) => m.encode(w),
            Self::ClipboardRequest(m) => m.encode(w),
            Self::Chunk(m) => m.encode(w),
            Self::Cancel(m) => m.encode(w),
            Self::FileOffer(m) => m.encode(w),
            Self::FileAccept { id } | Self::FileOpen { id } => id.encode(w),
            Self::FileCancel(m) => m.encode(w),
            Self::FileList(m) => m.encode(w),
            Self::FileResume(m) => m.encode(w),
            Self::FileData { data } => file::encode_data(data, w),
            Self::FileEnd(m) => m.encode(w),
            Self::FileDone | Self::OpenSettings | Self::InputBlocked => Ok(()),
        }
    }

    pub(crate) fn decode_body(ty: u16, r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(match ty {
            ty::SECURE_CHALLENGE => Self::SecureChallenge { nonce: r.array()? },
            ty::SECURE_INPUT => Self::SecureInput(InputProof::decode(r)?),
            ty::HELLO => Self::Hello(Hello::decode(r)?),
            ty::WELCOME => Self::Welcome(Welcome::decode(r)?),
            ty::REJECT => Self::Reject(Reject::decode(r)?),
            ty::BYE => Self::Bye(Bye::decode(r)?),
            ty::PING => Self::Ping { nonce: r.u64()? },
            ty::PONG => Self::Pong { nonce: r.u64()? },
            ty::PAIR_COMMIT => Self::PairCommit {
                commitment: r.array()?,
            },
            ty::PAIR_NONCE => Self::PairNonce { nonce: r.array()? },
            ty::PAIR_RESULT => Self::PairResult {
                accepted: r.bool()?,
            },
            ty::DISPLAYS => Self::Displays(r.list(MAX_DISPLAYS, "显示器数量")?),
            ty::ENTER => Self::Enter(Enter::decode(r)?),
            ty::LEAVE => Self::Leave { seq: r.u32()? },
            ty::PEER_SIDE => Self::PeerSide {
                side: Side::decode(r)?,
            },
            ty::DROP_HINT => Self::DropHint(DropHint::decode(r)?),
            ty::KEY => Self::Input(InputEvent::Key {
                code: KeyCode::decode(r)?,
                action: KeyAction::decode(r)?,
            }),
            ty::POINTER_MOTION => Self::Input(InputEvent::PointerMotion {
                position: Point::decode(r)?,
            }),
            ty::POINTER_DELTA => Self::Input(InputEvent::PointerDelta {
                dx: r.i32()?,
                dy: r.i32()?,
            }),
            ty::BUTTON => Self::Input(InputEvent::Button {
                button: MouseButton::decode(r)?,
                pressed: r.bool()?,
                position: Point::decode(r)?,
            }),
            ty::WHEEL => Self::Input(InputEvent::Wheel {
                dx: r.i32()?,
                dy: r.i32()?,
                position: Point::decode(r)?,
            }),
            ty::CLIPBOARD_OFFER => Self::ClipboardOffer(ClipboardOffer::decode(r)?),
            ty::CLIPBOARD_REQUEST => Self::ClipboardRequest(ClipboardRequest::decode(r)?),
            ty::CHUNK => Self::Chunk(Chunk::decode(r)?),
            ty::CANCEL => Self::Cancel(Cancel::decode(r)?),
            ty::FILE_OFFER => Self::FileOffer(FileOffer::decode(r)?),
            ty::FILE_ACCEPT => Self::FileAccept {
                id: TransferId::decode(r)?,
            },
            ty::FILE_CANCEL => Self::FileCancel(FileCancel::decode(r)?),
            ty::FILE_OPEN => Self::FileOpen {
                id: TransferId::decode(r)?,
            },
            ty::FILE_LIST => Self::FileList(FileList::decode(r)?),
            ty::FILE_RESUME => Self::FileResume(FileResume::decode(r)?),
            ty::FILE_DATA => Self::FileData {
                data: file::decode_data(r)?,
            },
            ty::FILE_END => Self::FileEnd(FileEnd::decode(r)?),
            ty::FILE_DONE => Self::FileDone,
            ty::OPEN_SETTINGS => Self::OpenSettings,
            ty::INPUT_BLOCKED => Self::InputBlocked,
            ty::APP_INFO => Self::AppInfo {
                version: r.str(MAX_VERSION_LEN, "版本号长度")?,
            },
            _ => return Err(DecodeError::UnknownMessage(ty)),
        })
    }
}

fn encode_input(event: &InputEvent, w: &mut Writer<'_>) -> Result<(), EncodeError> {
    match event {
        InputEvent::Key { code, action } => {
            code.encode(w)?;
            action.encode(w)
        }
        InputEvent::PointerMotion { position } => position.encode(w),
        InputEvent::PointerDelta { dx, dy } => {
            w.i32(*dx);
            w.i32(*dy);
            Ok(())
        }
        InputEvent::Button {
            button,
            pressed,
            position,
        } => {
            button.encode(w)?;
            w.bool(*pressed);
            position.encode(w)
        }
        InputEvent::Wheel { dx, dy, position } => {
            w.i32(*dx);
            w.i32(*dy);
            position.encode(w)
        }
    }
}
