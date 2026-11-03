use std::time::{Duration, Instant};

use ferry_trust::{
    Identity, InputAuthorization, InputProofVerifier, InputSigner, PROOF_LEASE, PROOF_LIFETIME,
};

fn signer() -> InputSigner {
    InputSigner::new(&Identity::generate("controller").unwrap()).unwrap()
}

#[test]
fn public_authorization_round_trips_without_private_material() {
    let signer = signer();
    let code = signer.authorization().to_code();
    assert_eq!(code.len(), 195);
    assert_eq!(
        InputAuthorization::from_code(&code).unwrap(),
        *signer.authorization()
    );
    for bad in ["", "S-1-5-18", &"界".repeat(65), &"0".repeat(195)] {
        assert!(InputAuthorization::from_code(bad).is_err());
    }
}

#[test]
fn every_authenticated_field_and_the_signer_are_verified() {
    let signer = signer();
    let now = Instant::now();
    let mut verifier = InputProofVerifier::new(signer.authorization().public_key, now);
    let nonce = verifier.challenge(now).unwrap();
    let proof = signer.sign(nonce, 1, [42; 32]).unwrap();
    for byte in 0..136 {
        let mut bad = proof;
        if byte < 32 {
            bad.challenge[byte] ^= 1;
        } else if byte < 40 {
            bad.sequence ^= 1 << ((byte - 32) * 8);
        } else if byte < 72 {
            bad.body[byte - 40] ^= 1;
        } else {
            bad.signature[byte - 72] ^= 1;
        }
        assert!(verifier.accept(&bad, now).is_err(), "byte {byte}");
    }
    let other = InputSigner::new(&Identity::generate("other").unwrap()).unwrap();
    assert!(
        verifier
            .accept(&other.sign(nonce, 1, proof.body).unwrap(), now)
            .is_err()
    );
    verifier.accept(&proof, now).unwrap();
    assert!(
        verifier.accept(&proof, now).is_err(),
        "同一证明只能执行一次"
    );
}

#[test]
fn challenges_expire_and_cannot_cross_connections_or_reset_sequences() {
    let signer = signer();
    let now = Instant::now();
    let key = signer.authorization().public_key;
    let mut first = InputProofVerifier::new(key, now);
    let nonce = first.challenge(now).unwrap();
    let proof = signer.sign(nonce, 1, [0; 32]).unwrap();
    let mut other = InputProofVerifier::new(key, now);
    assert_ne!(other.challenge(now).unwrap(), nonce);
    assert!(other.accept(&proof, now).is_err());
    first.accept(&proof, now).unwrap();
    let later = now + Duration::from_millis(500);
    let next = first.challenge(later).unwrap();
    assert_ne!(next, nonce);
    assert!(
        first
            .accept(&signer.sign(next, 1, [0; 32]).unwrap(), later)
            .is_err()
    );
    assert!(
        first
            .accept(&signer.sign(next, 3, [0; 32]).unwrap(), later)
            .is_err()
    );
    first
        .accept(&signer.sign(next, 2, [0; 32]).unwrap(), later)
        .unwrap();
    assert!(
        first
            .accept(
                &signer.sign(nonce, 3, [0; 32]).unwrap(),
                now + PROOF_LIFETIME
            )
            .is_err()
    );
}

#[test]
fn local_challenge_requests_never_renew_the_remote_lease() {
    let signer = signer();
    let now = Instant::now();
    let mut verifier = InputProofVerifier::new(signer.authorization().public_key, now);
    for millis in (0..1500).step_by(100) {
        verifier
            .challenge(now + Duration::from_millis(millis))
            .unwrap();
    }
    assert!(verifier.expired(now + PROOF_LEASE));
    assert!(verifier.challenge(now + PROOF_LEASE).is_err());
}

#[test]
fn signed_keepalives_renew_but_cannot_resurrect_authority() {
    let signer = signer();
    let now = Instant::now();
    let mut verifier = InputProofVerifier::new(signer.authorization().public_key, now);
    let nonce = verifier.challenge(now).unwrap();
    verifier
        .accept(
            &signer.sign(nonce, 1, [0; 32]).unwrap(),
            now + Duration::from_millis(500),
        )
        .unwrap();
    assert!(!verifier.expired(now + PROOF_LEASE));
    assert!(verifier.expired(now + Duration::from_secs(2)));
    assert!(
        verifier
            .accept(
                &signer.sign(nonce, 2, [0; 32]).unwrap(),
                now + Duration::from_secs(2)
            )
            .is_err()
    );
}
