//! 远端签署动作的唯一编码；32 字节正文只容纳键鼠、进入/离开和保活。

use ferry_types::{InputEvent, InputProof};

use crate::error::{DecodeError, EncodeError};
use crate::message::Message;
use crate::screen::Enter;
use crate::wire::{Body, Reader, Writer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecureAction {
    Alive,
    Enter(Enter),
    Leave { seq: u32 },
    Input(InputEvent),
}

impl SecureAction {
    pub fn from_message(message: &Message) -> Option<Self> {
        match message {
            Message::Enter(e) => Some(Self::Enter(*e)),
            Message::Leave { seq } => Some(Self::Leave { seq: *seq }),
            Message::Input(e) => Some(Self::Input(*e)),
            _ => None,
        }
    }

    pub fn message(self) -> Option<Message> {
        match self {
            Self::Alive => None,
            Self::Enter(e) => Some(Message::Enter(e)),
            Self::Leave { seq } => Some(Message::Leave { seq }),
            Self::Input(e) => Some(Message::Input(e)),
        }
    }

    pub fn encode(self) -> Result<[u8; 32], EncodeError> {
        let mut bytes = Vec::with_capacity(32);
        let mut w = Writer::new(&mut bytes);
        if let Some(message) = self.message() {
            w.u16(message.type_id());
            message.encode_body(&mut w)?;
        } else {
            w.u16(0);
        }
        let mut body = [0; 32];
        body[..bytes.len()].copy_from_slice(&bytes);
        Ok(body)
    }

    pub fn decode(body: &[u8; 32]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(body);
        let kind = r.u16()?;
        let action = if kind == 0 {
            Self::Alive
        } else {
            // 不分派未知类型，更不解析文件、路径或剪贴板消息。
            if !Message::is_input_type(kind) && !matches!(kind, 0x0101 | 0x0102) {
                return Err(DecodeError::InvalidValue("签署输入动作"));
            }
            let message = Message::decode_body(kind, &mut r)?;
            Self::from_message(&message).ok_or(DecodeError::InvalidValue("签署输入动作"))?
        };
        if action
            .encode()
            .map_err(|_| DecodeError::InvalidValue("签署输入正文"))?
            != *body
        {
            return Err(DecodeError::InvalidValue("签署输入填充"));
        }
        Ok(action)
    }
}

impl Body for InputProof {
    fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.array(&self.challenge);
        w.u64(self.sequence);
        w.array(&self.body);
        w.array(&self.signature);
        Ok(())
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            challenge: r.array()?,
            sequence: r.u64()?,
            body: r.array()?,
            signature: r.array()?,
        })
    }
}
