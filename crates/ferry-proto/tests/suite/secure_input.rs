use ferry_proto::{Capabilities, Channel, Codec, Enter, Message, SecureAction};
use ferry_types::{InputEvent, InputProof, KeyAction, KeyCode, LockKeys, Point};

#[test]
fn only_canonical_input_actions_can_be_signed() {
    for action in [
        SecureAction::Alive,
        SecureAction::Enter(Enter {
            seq: 4,
            position: Point::new(-1920, 200),
            locks: LockKeys::CAPS,
        }),
        SecureAction::Leave { seq: 4 },
        SecureAction::Input(InputEvent::Key {
            code: KeyCode::LEFT_CTRL,
            action: KeyAction::Down,
        }),
        SecureAction::Input(InputEvent::Wheel {
            dx: -120,
            dy: 120,
            position: Point::new(20, 40),
        }),
    ] {
        let body = action.encode().unwrap();
        assert_eq!(SecureAction::decode(&body).unwrap(), action);
        for byte in 0..32 {
            let mut changed = body;
            changed[byte] ^= 0x80;
            if let Ok(decoded) = SecureAction::decode(&changed) {
                assert_eq!(decoded.encode().unwrap(), changed);
            }
        }
    }
    for kind in [1_u16, 0x0107, 0x0108, 0x0200, 0x0412, 0xffff] {
        let mut body = [0; 32];
        body[..2].copy_from_slice(&kind.to_le_bytes());
        assert!(SecureAction::decode(&body).is_err());
    }
}

#[test]
fn proof_and_challenge_are_restricted_to_the_input_connection() {
    let proof = InputProof {
        challenge: [1; 32],
        sequence: 5,
        body: [0; 32],
        signature: [2; 64],
    };
    for message in [
        Message::SecureInput(proof),
        Message::SecureChallenge { nonce: [3; 32] },
    ] {
        let codec = Codec::new(Channel::Input);
        let mut bytes = Vec::new();
        codec.encode(&message, &mut bytes).unwrap();
        assert_eq!(codec.decode(&bytes).unwrap().unwrap().0, message);
        for channel in [Channel::Data, Channel::Pair, Channel::File, Channel::Invite] {
            assert!(Codec::new(channel).decode(&bytes).is_err());
            assert!(
                Codec::new(channel)
                    .encode(&message, &mut Vec::new())
                    .is_err()
            );
        }
    }
    assert_ne!(Capabilities::SECURE_INPUT.bits(), 0);
}
