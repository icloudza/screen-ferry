//! 远端输入的域分离签名、短时挑战与连续序号；不接受本机后台声明的输入来源。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ferry_types::InputProof;
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{
    ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair,
    UnparsedPublicKey,
};

use crate::{Error, Fingerprint, Identity, Result};

pub const PROOF_LIFETIME: Duration = Duration::from_secs(1);
pub const PROOF_LEASE: Duration = Duration::from_millis(1500);
const ROTATE: Duration = Duration::from_millis(250);
const DOMAIN: &[u8] = b"ScreenFerry secure input proof v2\0";

/// 必须通过可信渠道从控制端取得；不从被控端用户的配对文件导入。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputAuthorization {
    pub fingerprint: Fingerprint,
    pub public_key: [u8; 65],
}

impl InputAuthorization {
    pub fn to_code(&self) -> String {
        format!("{}:{}", self.fingerprint.to_hex(), hex(&self.public_key))
    }

    pub fn from_code(code: &str) -> Result<Self> {
        if code.len() != 195 || code.as_bytes().get(64) != Some(&b':') {
            return Err(Error::InputProof("授权码长度或分隔符无效"));
        }
        let fingerprint = code[..64].parse()?;
        let mut public_key = [0; 65];
        for (out, pair) in public_key
            .iter_mut()
            .zip(code.as_bytes()[65..].chunks_exact(2))
        {
            *out = std::str::from_utf8(pair)
                .ok()
                .and_then(|p| u8::from_str_radix(p, 16).ok())
                .ok_or(Error::InputProof("授权公钥编码无效"))?;
        }
        if public_key[0] != 4 {
            return Err(Error::InputProof("需要未压缩 P-256 公钥"));
        }
        Ok(Self {
            fingerprint,
            public_key,
        })
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct InputSigner {
    key: EcdsaKeyPair,
    authorization: InputAuthorization,
}

impl InputSigner {
    pub fn new(identity: &Identity) -> Result<Self> {
        let key = EcdsaKeyPair::from_pkcs8(
            &ECDSA_P256_SHA256_FIXED_SIGNING,
            identity.key_der(),
            &SystemRandom::new(),
        )
        .map_err(|_| Error::InputProof("设备输入签名私钥无效"))?;
        let public_key = key
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| Error::InputProof("设备需要 P-256 公钥"))?;
        Ok(Self {
            key,
            authorization: InputAuthorization {
                fingerprint: identity.fingerprint(),
                public_key,
            },
        })
    }

    pub fn authorization(&self) -> &InputAuthorization {
        &self.authorization
    }

    pub fn sign(&self, challenge: [u8; 32], sequence: u64, body: [u8; 32]) -> Result<InputProof> {
        let mut proof = InputProof {
            challenge,
            sequence,
            body,
            signature: [0; 64],
        };
        let signature = self
            .key
            .sign(&SystemRandom::new(), &signed_bytes(&proof))
            .map_err(|_| Error::Random)?;
        proof.signature.copy_from_slice(signature.as_ref());
        Ok(proof)
    }
}

fn signed_bytes(proof: &InputProof) -> [u8; DOMAIN.len() + 72] {
    let mut bytes = [0; DOMAIN.len() + 72];
    bytes[..DOMAIN.len()].copy_from_slice(DOMAIN);
    let body = &mut bytes[DOMAIN.len()..];
    body[..32].copy_from_slice(&proof.challenge);
    body[32..40].copy_from_slice(&proof.sequence.to_le_bytes());
    body[40..].copy_from_slice(&proof.body);
    bytes
}

/// 实例只属于一条已授权管道；本机检查不会调用 accept，不能续期。
pub struct InputProofVerifier {
    key: [u8; 65],
    challenges: VecDeque<([u8; 32], Instant)>,
    issued: Option<Instant>,
    next: Option<u64>,
    deadline: Instant,
}

impl InputProofVerifier {
    pub fn new(public_key: [u8; 65], now: Instant) -> Self {
        Self {
            key: public_key,
            challenges: VecDeque::new(),
            issued: None,
            next: Some(1),
            deadline: now + PROOF_LEASE,
        }
    }

    pub fn expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }

    pub fn challenge(&mut self, now: Instant) -> Result<[u8; 32]> {
        if self.expired(now) {
            return Err(Error::InputProof("远端输入租约到期"));
        }
        while self
            .challenges
            .front()
            .is_some_and(|(_, until)| now >= *until)
        {
            self.challenges.pop_front();
        }
        if self
            .issued
            .is_none_or(|last| now.saturating_duration_since(last) >= ROTATE)
        {
            let mut nonce = [0; 32];
            SystemRandom::new()
                .fill(&mut nonce)
                .map_err(|_| Error::Random)?;
            self.challenges.push_back((nonce, now + PROOF_LIFETIME));
            self.issued = Some(now);
        }
        Ok(self.challenges.back().expect("已生成挑战").0)
    }

    pub fn accept(&mut self, proof: &InputProof, now: Instant) -> Result<()> {
        if self.expired(now)
            || self.next != Some(proof.sequence)
            || !self
                .challenges
                .iter()
                .any(|(nonce, until)| nonce == &proof.challenge && now < *until)
        {
            return Err(Error::InputProof("输入证明过期、乱序或不属于当前会话"));
        }
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, self.key)
            .verify(&signed_bytes(proof), &proof.signature)
            .map_err(|_| Error::InputProof("输入签名无效"))?;
        self.next = proof.sequence.checked_add(1);
        self.deadline = now + PROOF_LEASE;
        Ok(())
    }
}
