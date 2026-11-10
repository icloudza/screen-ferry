use ferry_net::{Error, Framed, accept, initiate};
use ferry_proto::{Capabilities, Channel, Hello, RejectReason, WIRE_FORMAT};
use tokio::io::duplex;

#[tokio::test]
async fn both_sides_agree_on_the_session() {
    let (a, b) = duplex(1024);
    let mut client = Framed::new(a, Channel::Input);
    let mut server = Framed::new(b, Channel::Input);

    let hello = Hello::new(Channel::Input, Capabilities::all(), 0, "Mac");
    let (welcome, accepted) = tokio::join!(
        initiate(&mut client, hello),
        accept(&mut server, Capabilities::CLIPBOARD, "PC", |_| Ok(42)),
    );
    let welcome = welcome.unwrap();
    let (hello, sent) = accepted.unwrap();

    assert_eq!(welcome, sent);
    assert_eq!(welcome.session, 42);
    assert_eq!(welcome.capabilities, Capabilities::CLIPBOARD);
    assert_eq!(welcome.name, "PC");
    assert_eq!(hello.name, "Mac");
}

#[tokio::test]
async fn version_mismatch_is_rejected_on_both_sides() {
    let (a, b) = duplex(1024);
    let mut client = Framed::new(a, Channel::Input);
    let mut server = Framed::new(b, Channel::Input);

    let hello = Hello {
        format: WIRE_FORMAT + 1,
        ..Hello::new(Channel::Input, Capabilities::empty(), 0, "新版")
    };
    let (initiated, accepted) = tokio::join!(
        initiate(&mut client, hello),
        accept(&mut server, Capabilities::empty(), "旧版", |_| Ok(1))
    );

    for result in [initiated.map(drop), accepted.map(drop)] {
        match result {
            Err(Error::Rejected(r)) => assert_eq!(r.reason, RejectReason::INCOMPATIBLE_FORMAT),
            other => panic!("应被拒绝，实际 {other:?}"),
        }
    }
}

#[tokio::test]
async fn the_acceptor_can_reject_after_reading_the_hello() {
    let (a, b) = duplex(1024);
    let mut client = Framed::new(a, Channel::Data);
    let mut server = Framed::new(b, Channel::Data);

    let hello = Hello::new(Channel::Data, Capabilities::CLIPBOARD, 7, "Mac");
    let (initiated, accepted) = tokio::join!(
        initiate(&mut client, hello),
        accept(&mut server, Capabilities::CLIPBOARD, "PC", |hello| {
            assert_eq!((hello.channel, hello.session), (Channel::Data, 7));
            Err(RejectReason::UNKNOWN_SESSION)
        })
    );

    for result in [initiated.map(drop), accepted.map(drop)] {
        match result {
            Err(Error::Rejected(r)) => assert_eq!(r.reason, RejectReason::UNKNOWN_SESSION),
            other => panic!("应被拒绝，实际 {other:?}"),
        }
    }
}
