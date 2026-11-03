//! 跨平台传递的远端输入证明；正文编码和签名验证分别由 proto、trust 负责。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputProof {
    pub challenge: [u8; 32],
    pub sequence: u64,
    pub body: [u8; 32],
    pub signature: [u8; 64],
}
