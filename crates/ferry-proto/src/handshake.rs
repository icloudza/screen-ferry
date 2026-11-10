//! 握手：格式标记、连接类型、能力协商，以及拒绝与告别。
//!
//! 每条连接建立后，发起方先发 [`Hello`]；接受方回 [`Welcome`] 或 [`Reject`]。
//! 这三条消息的格式永久不变，新旧版本的双方总能完成协商。

use std::fmt;

use bitflags::bitflags;

use crate::error::{DecodeError, EncodeError};
use crate::limits::MAX_NAME_LEN;
use crate::wire::{Body, Reader, Writer};

/// 握手开头的格式标记，固定为 1，不要改。
///
/// 它不是一个会演进的“协议版本”：软件只有一个版本号，新功能一律用能力位协商（见 [`Capabilities`]），
/// 新旧版本因此总能互连。已经发布的版本握手时要求这个值相等，局域网广播里也带着它，
/// 改掉它就等于让所有旧版本连不上。只有帧格式或握手本身不得不变时才动它，那时两台必须一起更新。
pub const WIRE_FORMAT: u16 = 1;

/// 连接类型。每对设备之间各有一条。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    /// 键鼠事件、屏幕切换、心跳：小而急，不能被堵。
    Input,
    /// 剪贴板与文件：大而慢，分块传输。
    Data,
    /// 配对：首次连接时交换配对码所需的随机数，完成后即关闭。
    Pair,
    /// 文件：一次文件传输一条，传完即关闭。
    File,
    /// 邀请：服务端请客户端连过来。客户端在单独的端口上等它；`Hello` 的 `session` 填服务端监听的端口，
    /// 客户端回 `Welcome`（或者正连着别的服务端时回 `Reject(BUSY)`）后关闭，再照常连接服务端。
    Invite,
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Input => "输入",
            Self::Data => "数据",
            Self::Pair => "配对",
            Self::File => "文件",
            Self::Invite => "邀请",
        })
    }
}

bitflags! {
    /// 可选能力。双方取交集，只使用都支持的能力。
    ///
    /// 解码时保留未知的位，协商时自然被交集去掉。
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Capabilities: u64 {
        /// 光标相对移动（游戏等锁定光标的程序）。
        const RELATIVE_POINTER = 1 << 0;
        /// 剪贴板同步。
        const CLIPBOARD = 1 << 1;
        /// 文件传输。
        const FILES = 1 << 2;
        /// 边缘投递：告诉对方相邻的方向，拖动中提示对方提前滑出接收胶囊。
        const EDGE_DROP = 1 << 3;
        /// 本机的快捷键以 Command 键为主（macOS）。这一位说的是本机是什么键盘，不是一项能力：
        /// 服务端看客户端 `Hello` 里有没有它，决定要不要互换 Command 与 Ctrl，不看交集。
        const COMMAND_KEY = 1 << 4;
        /// 能按对方的请求打开本机的设置界面（`OpenSettings` 消息）。
        const REMOTE_SETTINGS = 1 << 5;
        /// 握手后互相告诉程序版本（`AppInfo` 消息）。
        const APP_INFO = 1 << 6;
        /// 代理独立验证由控制端设备私钥签署的输入。
        const SECURE_INPUT = 1 << 7;
        /// 被控端收不了键鼠时请控制端收回光标（`InputBlocked` 消息），不必断开连接。
        const INPUT_BLOCKED = 1 << 8;
    }
}

/// 发起方的问候。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hello {
    /// 格式标记，见 [`WIRE_FORMAT`]。
    pub format: u16,
    pub channel: Channel,
    pub capabilities: Capabilities,
    /// 输入连接填 0；数据连接和文件连接填输入连接握手时拿到的会话号。
    pub session: u64,
    /// 发起方的设备名，用于显示。
    pub name: String,
}

impl Hello {
    /// 当前版本的问候。
    pub fn new(
        channel: Channel,
        capabilities: Capabilities,
        session: u64,
        name: impl Into<String>,
    ) -> Self {
        Self {
            format: WIRE_FORMAT,
            channel,
            capabilities,
            session,
            name: name.into(),
        }
    }

    /// 作为接受方回应：检查格式标记，并协商出双方共同的能力。
    ///
    /// 数据连接的会话号是否有效，需要由调用方另行检查。
    pub fn accept(
        &self,
        capabilities: Capabilities,
        session: u64,
        name: impl Into<String>,
    ) -> Result<Welcome, Reject> {
        if self.format != WIRE_FORMAT {
            return Err(Reject::new(RejectReason::INCOMPATIBLE_FORMAT));
        }
        Ok(Welcome {
            format: WIRE_FORMAT,
            capabilities: self.capabilities & capabilities,
            session,
            name: name.into(),
        })
    }
}

/// 接受方的欢迎。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Welcome {
    /// 格式标记，见 [`WIRE_FORMAT`]。
    pub format: u16,
    /// 协商后的能力，即双方能力的交集。
    pub capabilities: Capabilities,
    /// 本次会话号。数据连接握手时要带上它。
    pub session: u64,
    /// 接受方的设备名，用于显示。
    pub name: String,
}

/// 接受方的拒绝。发出后关闭连接。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reject {
    pub reason: RejectReason,
    /// 接受方的格式标记（见 [`WIRE_FORMAT`]）。
    pub format: u16,
}

impl Reject {
    pub const fn new(reason: RejectReason) -> Self {
        Self {
            reason,
            format: WIRE_FORMAT,
        }
    }
}

/// 拒绝原因。以后可能新增，所以用编号表示。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RejectReason(pub u16);

impl RejectReason {
    /// 格式标记不同（见 [`WIRE_FORMAT`]）：两台差了一次不兼容的改动，要一起更新。
    pub const INCOMPATIBLE_FORMAT: Self = Self(1);
    /// 数据连接带来的会话号不存在。
    pub const UNKNOWN_SESSION: Self = Self(2);
    /// 已被其他设备控制，暂不接受新连接。
    pub const BUSY: Self = Self(3);
    /// 对方证书不在信任列表中，需要先配对。
    pub const NOT_PAIRED: Self = Self(4);
}

/// 主动关闭连接前的告别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bye {
    pub reason: ByeReason,
}

/// 告别原因。以后可能新增，所以用编号表示。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ByeReason(pub u16);

impl ByeReason {
    /// 正常关闭：退出程序、断开设备。
    pub const CLOSED: Self = Self(0);
    /// 同一设备建立了新连接，旧连接被取代。
    pub const REPLACED: Self = Self(1);
    /// 收到了违反协议的数据。
    pub const PROTOCOL_ERROR: Self = Self(2);
    /// 心跳超时。
    pub const TIMEOUT: Self = Self(3);
}

impl Body for Channel {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u8(match self {
            Self::Input => 1,
            Self::Data => 2,
            Self::Pair => 3,
            Self::File => 4,
            Self::Invite => 5,
        });
        Ok(())
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match r.u8()? {
            1 => Ok(Self::Input),
            2 => Ok(Self::Data),
            3 => Ok(Self::Pair),
            4 => Ok(Self::File),
            5 => Ok(Self::Invite),
            _ => Err(DecodeError::InvalidValue("连接类型")),
        }
    }
}

impl Body for Capabilities {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u64(self.bits());
        Ok(())
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        r.u64().map(Self::from_bits_retain)
    }
}

impl Body for Hello {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u16(self.format);
        self.channel.encode(w)?;
        self.capabilities.encode(w)?;
        w.u64(self.session);
        w.str(&self.name, MAX_NAME_LEN, "设备名长度")
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            format: r.u16()?,
            channel: Channel::decode(r)?,
            capabilities: Capabilities::decode(r)?,
            session: r.u64()?,
            name: r.str(MAX_NAME_LEN, "设备名长度")?,
        })
    }
}

impl Body for Welcome {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u16(self.format);
        self.capabilities.encode(w)?;
        w.u64(self.session);
        w.str(&self.name, MAX_NAME_LEN, "设备名长度")
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            format: r.u16()?,
            capabilities: Capabilities::decode(r)?,
            session: r.u64()?,
            name: r.str(MAX_NAME_LEN, "设备名长度")?,
        })
    }
}

impl Body for Reject {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u16(self.reason.0);
        w.u16(self.format);
        Ok(())
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            reason: RejectReason(r.u16()?),
            format: r.u16()?,
        })
    }
}

impl Body for Bye {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u16(self.reason.0);
        Ok(())
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            reason: ByeReason(r.u16()?),
        })
    }
}
