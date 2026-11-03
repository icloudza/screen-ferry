//! 信任相关的错误。

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("无法生成设备证书：{0}")]
    Generate(#[from] rcgen::Error),
    #[error("设备身份无效：{0}")]
    InvalidIdentity(&'static str),
    #[error("指纹格式不对：应为 64 位十六进制")]
    InvalidFingerprint,
    #[error("系统随机数不可用")]
    Random,
    #[error("安全输入认证失败：{0}")]
    InputProof(&'static str),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
