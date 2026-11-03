//! 设备信任：设备身份、证书指纹、配对码校验、TLS 证书验证器。
//!
//! 每台设备有一张自签名证书作为身份，用证书的 SHA-256 指纹识别对方。
//! 首次连接时双方按 [`pairing`] 中的流程核对配对码，确认后把对方指纹加入 [`PeerList`]，
//! 以后只认这张证书。TLS 层用 [`AnyPeerVerifier`] 接受任何证书但校验握手签名，
//! 证书是否可信由上层按指纹判断。
//!
//! 层级：L1。只做计算，身份与信任列表的持久化由 ferry-store 负责。

#![forbid(unsafe_code)]

pub mod pairing;

mod error;
mod fingerprint;
mod identity;
mod input_proof;
mod peers;
mod verifier;

pub use error::{Error, Result};
pub use fingerprint::Fingerprint;
pub use identity::Identity;
pub use input_proof::{
    InputAuthorization, InputProofVerifier, InputSigner, PROOF_LEASE, PROOF_LIFETIME,
};
pub use peers::{Peer, PeerList};
pub use verifier::AnyPeerVerifier;
