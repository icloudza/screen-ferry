//! Windows 特权输入代理的独立本机协议。定长 192 字节，无路径、字符串或任意执行请求。

use ferry_types::{InputEvent, InputProof, LockKeys};

use crate::error::{DecodeError, EncodeError};
use crate::message::Message;
use crate::wire::{Body, Reader, Writer};

pub const FRAME_LEN: usize = 192;
const VERSION: u16 = 2;
const HEADER: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Check,
    Input(InputEvent),
    Locks { wanted: LockKeys, mask: LockKeys },
    Cursor { hidden: bool },
    Release,
    Bind { peer: [u8; 32] },
    Verified(InputProof),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub sequence: u64,
    /// GetTickCount64 的绝对期限；接收及执行前都要检查。
    pub expires_ms: u64,
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Unsupported,
    Unavailable,
    Expired,
    Failed,
    Unauthorized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Response {
    pub sequence: u64,
    pub status: Status,
    pub challenge: Option<[u8; 32]>,
}

impl Request {
    pub fn encode(self) -> Result<[u8; FRAME_LEN], EncodeError> {
        let mut payload = Vec::with_capacity(24);
        let mut w = Writer::new(&mut payload);
        let kind = match self.command {
            Command::Check => 1,
            Command::Input(event) => {
                let message = Message::Input(event);
                w.u16(message.type_id());
                message.encode_body(&mut w)?;
                2
            }
            Command::Locks { wanted, mask } => {
                wanted.encode(&mut w)?;
                mask.encode(&mut w)?;
                3
            }
            Command::Cursor { hidden } => {
                w.bool(hidden);
                4
            }
            Command::Release => 5,
            Command::Bind { peer } => {
                w.array(&peer);
                6
            }
            Command::Verified(proof) => {
                proof.encode(&mut w)?;
                7
            }
        };
        Ok(frame(
            *b"SFIN",
            self.sequence,
            self.expires_ms,
            kind,
            &payload,
        ))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let (sequence, expires_ms, kind, payload) = header(bytes, *b"SFIN")?;
        let mut r = Reader::new(payload);
        let command = match kind {
            1 => Command::Check,
            2 => {
                let kind = r.u16()?;
                if !Message::is_input_type(kind) {
                    return Err(DecodeError::InvalidValue("输入代理只接受键鼠事件"));
                }
                let Message::Input(event) = Message::decode_body(kind, &mut r)? else {
                    return Err(DecodeError::InvalidValue("输入事件"));
                };
                Command::Input(event)
            }
            3 => Command::Locks {
                wanted: LockKeys::decode(&mut r)?,
                mask: LockKeys::decode(&mut r)?,
            },
            4 => Command::Cursor { hidden: r.bool()? },
            5 => Command::Release,
            6 => Command::Bind { peer: r.array()? },
            7 => Command::Verified(InputProof::decode(&mut r)?),
            _ => return Err(DecodeError::InvalidValue("输入代理命令")),
        };
        r.finish()?;
        Ok(Self {
            sequence,
            expires_ms,
            command,
        })
    }
}

impl Response {
    pub fn encode(self) -> [u8; FRAME_LEN] {
        let status = match self.status {
            Status::Ok => 0,
            Status::Unsupported => 1,
            Status::Unavailable => 2,
            Status::Expired => 3,
            Status::Failed => 4,
            Status::Unauthorized => 5,
        };
        let mut payload = vec![status, u8::from(self.challenge.is_some())];
        if let Some(challenge) = self.challenge {
            payload.extend_from_slice(&challenge);
        }
        frame(*b"SFAR", self.sequence, 0, 128, &payload)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let (sequence, reserved, kind, payload) = header(bytes, *b"SFAR")?;
        if reserved != 0 || kind != 128 {
            return Err(DecodeError::InvalidValue("输入代理回复"));
        }
        let mut r = Reader::new(payload);
        let status = match r.u8()? {
            0 => Status::Ok,
            1 => Status::Unsupported,
            2 => Status::Unavailable,
            3 => Status::Expired,
            4 => Status::Failed,
            5 => Status::Unauthorized,
            _ => return Err(DecodeError::InvalidValue("输入代理状态")),
        };
        let challenge = if r.bool()? { Some(r.array()?) } else { None };
        r.finish()?;
        if status != Status::Ok && challenge.is_some() {
            return Err(DecodeError::InvalidValue("失败回复不能提供挑战"));
        }
        Ok(Self {
            sequence,
            status,
            challenge,
        })
    }
}

fn frame(magic: [u8; 4], sequence: u64, expires: u64, kind: u8, payload: &[u8]) -> [u8; FRAME_LEN] {
    let mut frame = [0; FRAME_LEN];
    frame[..4].copy_from_slice(&magic);
    frame[4..6].copy_from_slice(&VERSION.to_le_bytes());
    frame[6..8].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    frame[8..16].copy_from_slice(&sequence.to_le_bytes());
    frame[16..24].copy_from_slice(&expires.to_le_bytes());
    frame[24] = kind;
    frame[HEADER..HEADER + payload.len()].copy_from_slice(payload);
    frame
}

fn header(bytes: &[u8], magic: [u8; 4]) -> Result<(u64, u64, u8, &[u8]), DecodeError> {
    if bytes.len() != FRAME_LEN {
        return Err(DecodeError::InvalidValue("输入代理帧长度"));
    }
    let mut r = Reader::new(&bytes[..HEADER]);
    if r.array::<4>()? != magic || r.u16()? != VERSION {
        return Err(DecodeError::InvalidValue("输入代理协议版本"));
    }
    let len = usize::from(r.u16()?);
    if len > FRAME_LEN - HEADER || bytes[HEADER + len..].iter().any(|&b| b != 0) {
        return Err(DecodeError::InvalidValue("输入代理消息长度或填充"));
    }
    Ok((r.u64()?, r.u64()?, r.u8()?, &bytes[HEADER..HEADER + len]))
}
