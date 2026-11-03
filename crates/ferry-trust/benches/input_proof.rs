//! 只测输入证明的签名/验证计算成本，不代表网络、IPC 或 Windows 原生注入延迟。

use std::hint::black_box;
use std::time::{Duration, Instant};

use ferry_trust::{Identity, InputProofVerifier, InputSigner};

fn main() {
    const SAMPLES: u64 = 20_000;
    let identity = Identity::generate("benchmark").unwrap();
    let signer = InputSigner::new(&identity).unwrap();
    let now = Instant::now();
    let mut verifier = InputProofVerifier::new(signer.authorization().public_key, now);
    let challenge = verifier.challenge(now).unwrap();
    let mut signing = Duration::ZERO;
    let mut verifying = Duration::ZERO;
    for sequence in 1..=SAMPLES + 256 {
        let start = Instant::now();
        let proof = signer
            .sign(black_box(challenge), sequence, black_box([0; 32]))
            .unwrap();
        let signed = start.elapsed();
        let start = Instant::now();
        // 固定逻辑时间仅隔离计算成本，不把租约、睡眠或网络时间混入基准。
        verifier.accept(black_box(&proof), now).unwrap();
        let verified = start.elapsed();
        if sequence > 256 {
            signing += signed;
            verifying += verified;
        }
    }
    let sign_us = signing.as_secs_f64() * 1_000_000.0 / SAMPLES as f64;
    let verify_us = verifying.as_secs_f64() * 1_000_000.0 / SAMPLES as f64;
    println!("{SAMPLES} proofs; signing mean {sign_us:.2} us; verification mean {verify_us:.2} us");
    println!("crypto only; excludes encoding, TLS, IPC, policy queries and native input");
}
