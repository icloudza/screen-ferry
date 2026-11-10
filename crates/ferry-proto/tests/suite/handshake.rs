use ferry_proto::{
    Capabilities, Channel, Codec, Hello, Message, Reject, RejectReason, WIRE_FORMAT,
};

use crate::support::encode;

#[test]
fn accept_negotiates_common_capabilities() {
    let hello = Hello::new(
        Channel::Input,
        Capabilities::CLIPBOARD | Capabilities::RELATIVE_POINTER,
        0,
        "Mac",
    );
    let welcome = hello.accept(Capabilities::CLIPBOARD, 77, "PC").unwrap();

    assert_eq!(welcome.format, WIRE_FORMAT);
    assert_eq!(welcome.capabilities, Capabilities::CLIPBOARD);
    assert_eq!(welcome.session, 77);
    assert_eq!(welcome.name, "PC");
}

#[test]
fn accept_drops_capabilities_it_does_not_know() {
    let from_newer_peer = Capabilities::from_bits_retain(1 << 63) | Capabilities::CLIPBOARD;
    let hello = Hello::new(Channel::Input, from_newer_peer, 0, "Mac");
    let welcome = hello.accept(Capabilities::all(), 1, "PC").unwrap();
    assert_eq!(welcome.capabilities, Capabilities::CLIPBOARD);
}

#[test]
fn accept_rejects_other_versions() {
    let hello = Hello {
        format: WIRE_FORMAT + 1,
        ..Hello::new(Channel::Input, Capabilities::all(), 0, "Mac")
    };
    assert_eq!(
        hello.accept(Capabilities::all(), 1, "PC"),
        Err(Reject {
            reason: RejectReason::INCOMPATIBLE_FORMAT,
            format: WIRE_FORMAT
        }),
    );
}

/// 握手消息的格式与版本无关：任何版本的 Hello 都能解出版本号，才能正确拒绝。
#[test]
fn hello_from_any_version_still_decodes() {
    let future = Message::Hello(Hello {
        format: 999,
        ..Hello::new(Channel::Data, Capabilities::all(), 5, "未来版本")
    });
    let buf = encode(Channel::Data, &future);
    let (decoded, _) = Codec::new(Channel::Data).decode(&buf).unwrap().unwrap();
    assert_eq!(decoded, future);
}

/// 0.1.0 握手时要求这个值等于 1，局域网广播里也带着它：改掉它，所有已发布的版本都连不上。
/// 新功能用能力位，不动这个值。
#[test]
fn the_format_marker_stays_one_forever() {
    assert_eq!(WIRE_FORMAT, 1);
}

#[test]
fn older_and_newer_peers_negotiate_down_to_what_both_understand() {
    // 0.1.0 只认识到 APP_INFO 为止的能力位。
    let v0_1_0 = Capabilities::CLIPBOARD
        | Capabilities::FILES
        | Capabilities::EDGE_DROP
        | Capabilities::REMOTE_SETTINGS
        | Capabilities::APP_INFO;

    // 旧版本来连新版本：谈出来的就是旧版本会的那些，新消息一条都不会发给它。
    let old = Hello::new(Channel::Input, v0_1_0, 0, "0.1.0");
    let welcome = old
        .accept(Capabilities::all(), 1, "新版本")
        .expect("照常接受");
    assert_eq!(welcome.capabilities, v0_1_0);

    // 将来的版本带着现在还不认识的能力位来连：不认识的位被交集去掉，照常接受。
    let future = Hello::new(
        Channel::Input,
        Capabilities::from_bits_retain(u64::MAX),
        0,
        "将来的版本",
    );
    let welcome = future
        .accept(Capabilities::all(), 1, "现在的版本")
        .expect("照常接受");
    assert_eq!(welcome.capabilities, Capabilities::all());
}
