use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ferry_engine::{Error, Event, Events, Files, TransferState, TransferUpdate};
use ferry_types::TransferId;
use tokio::time::sleep;

use crate::support::{Device, Rig, Setup, TempDir, eventually};

/// 把文件传输事件收集到一个列表里。
fn collect() -> (Events, Arc<Mutex<Vec<TransferUpdate>>>) {
    let (events, mut rx) = Events::channel();
    let seen = Arc::new(Mutex::new(Vec::new()));
    tokio::spawn({
        let seen = seen.clone();
        async move {
            while let Some(event) = rx.recv().await {
                if let Event::Transfer(update) = event {
                    seen.lock().unwrap().push(update);
                }
            }
        }
    });
    (events, seen)
}

fn reached(seen: &Mutex<Vec<TransferUpdate>>, pred: impl Fn(&TransferState) -> bool) -> bool {
    seen.lock().unwrap().iter().any(|u| pred(&u.state))
}

/// 连上之后再发：刚启动时数据连接可能还没建好。
async fn send_when_ready(files: &Files, paths: Vec<PathBuf>) -> TransferId {
    for _ in 0..300 {
        let Some(peer) = files.connected().first().copied() else {
            sleep(Duration::from_millis(10)).await;
            continue;
        };
        match files.send(peer, paths.clone()).await {
            Ok(id) => return id,
            Err(Error::NotConnected) => sleep(Duration::from_millis(10)).await,
            Err(e) => panic!("发送失败：{e}"),
        }
    }
    panic!("等待超时：数据连接没有建好");
}

fn received(device: &Device) -> PathBuf {
    device.store.downloads_dir()
}

fn write(path: &Path, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

async fn transfer_without_edge_drop(linux_server: bool) {
    let setup = if linux_server {
        Setup {
            mac_platform: "linux",
            pc_platform: "macos",
            mac_edge_drop: false,
            pc_edge_drop: true,
            ..Setup::default()
        }
    } else {
        Setup {
            mac_platform: "macos",
            pc_platform: "linux",
            mac_edge_drop: true,
            pc_edge_drop: false,
            ..Setup::default()
        }
    };
    let rig = Rig::with(setup).await;
    let source = TempDir::new();
    let file = source.path().join("跨系统.txt");
    let content = pattern(130_000);
    write(&file, &content);

    let (files, target) = if linux_server {
        (&rig.mac_files, received(&rig.pc))
    } else {
        (&rig.pc_files, received(&rig.mac))
    };
    send_when_ready(files, vec![file]).await;
    let got = target.join("跨系统.txt");
    eventually("没有边缘投递也能收到文件", || got.is_file()).await;
    assert_eq!(fs::read(got).unwrap(), content);
}

/// 每个字节都不一样的内容，错位、截断都能看出来。
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 % 251) as u8).collect()
}

#[tokio::test]
async fn linux_server_without_edge_drop_still_sends_files_to_macos() {
    transfer_without_edge_drop(true).await;
}

#[tokio::test]
async fn linux_client_without_edge_drop_still_sends_files_to_macos() {
    transfer_without_edge_drop(false).await;
}

#[tokio::test]
async fn a_file_goes_from_the_client_to_the_server() {
    let (mac_events, mac) = collect();
    let (pc_events, pc) = collect();
    let rig = Rig::with(Setup {
        mac_events,
        pc_events,
        ..Setup::default()
    })
    .await;
    let source = TempDir::new();
    let file = source.path().join("季度报告.pdf");
    // 跨过好几个分块，最后一块不满。
    let content = pattern(200_000);
    write(&file, &content);

    send_when_ready(&rig.pc_files, vec![file]).await;

    let target = received(&rig.mac).join("季度报告.pdf");
    eventually("服务端收到文件", || target.is_file()).await;
    assert_eq!(fs::read(&target).unwrap(), content);
    eventually("两边都报告完成", || {
        reached(&pc, |s| matches!(s, TransferState::Done { .. }))
            && reached(
                &mac,
                |s| matches!(s, TransferState::Done { saved } if saved == &vec![target.clone()]),
            )
    })
    .await;
    let partial = rig.mac.store.partial_dir();
    eventually("临时文件已清理", || {
        fs::read_dir(&partial).map_or(true, |mut d| d.next().is_none())
    })
    .await;
}

#[tokio::test]
async fn a_folder_goes_from_the_server_to_the_client() {
    let rig = Rig::start().await;
    let source = TempDir::new();
    let root = source.path().join("设计稿");
    write(&root.join("首页.png"), &pattern(70_000));
    write(&root.join("空文件.txt"), b"");
    // Windows 本身不允许文件名里有冒号，那边造不出这个源文件；清洗规则另有单元测试。
    let colon = std::env::consts::OS != "windows";
    if colon {
        write(&root.join("图标/a:b.svg"), b"<svg/>");
    }
    write(&root.join("图标/深/层/说明.md"), "层层叠叠".as_bytes());
    write(&root.join(".DS_Store"), b"junk");
    fs::create_dir_all(root.join("空文件夹")).unwrap();
    let lone = source.path().join("readme.txt");
    write(&lone, b"hi");

    send_when_ready(&rig.mac_files, vec![root, lone]).await;

    let got = received(&rig.pc);
    eventually("客户端收到所有项目", || {
        got.join("readme.txt").is_file()
    })
    .await;
    let folder = got.join("设计稿");
    assert_eq!(fs::read(folder.join("首页.png")).unwrap(), pattern(70_000));
    assert_eq!(fs::read(folder.join("空文件.txt")).unwrap(), b"");
    if colon {
        assert_eq!(
            fs::read(folder.join("图标/a_b.svg")).unwrap(),
            b"<svg/>",
            "冒号在 Windows 上不能用"
        );
    }
    assert_eq!(
        fs::read_to_string(folder.join("图标/深/层/说明.md")).unwrap(),
        "层层叠叠"
    );
    assert!(folder.join("空文件夹").is_dir(), "空文件夹也带过去");
    assert!(!folder.join(".DS_Store").exists(), "系统生成的文件不发送");
}

#[tokio::test]
async fn the_same_name_twice_gets_a_number() {
    let rig = Rig::start().await;
    let source = TempDir::new();
    let file = source.path().join("证书.p12");
    write(&file, b"first");

    send_when_ready(&rig.pc_files, vec![file.clone()]).await;
    let got = received(&rig.mac);
    eventually("第一份", || got.join("证书.p12").is_file()).await;

    write(&file, b"second");
    send_when_ready(&rig.pc_files, vec![file]).await;
    eventually("第二份自动编号", || {
        got.join("证书 (2).p12").is_file()
    })
    .await;
    assert_eq!(fs::read(got.join("证书.p12")).unwrap(), b"first");
    assert_eq!(fs::read(got.join("证书 (2).p12")).unwrap(), b"second");
}

#[tokio::test]
async fn cancelling_stops_both_sides_and_leaves_nothing_behind() {
    let (mac_events, mac) = collect();
    let (pc_events, pc) = collect();
    let rig = Rig::with(Setup {
        mac_events,
        pc_events,
        ..Setup::default()
    })
    .await;
    let source = TempDir::new();
    let file = source.path().join("发布录屏.mov");
    write(&file, &pattern(8 << 20));

    let id = send_when_ready(&rig.pc_files, vec![file]).await;
    rig.pc_files.cancel(id);

    eventually("发送方报告已取消", || {
        reached(&pc, |s| {
            matches!(s, TransferState::Cancelled { by_peer: false })
        })
    })
    .await;
    // 对方可能还没开始收，也可能收到一半；都不能留下文件。
    sleep(Duration::from_millis(300)).await;
    assert!(!received(&rig.mac).join("发布录屏.mov").exists());
    assert!(!reached(&mac, |s| matches!(s, TransferState::Done { .. })));
    let partial = rig.mac.store.partial_dir().join(id.to_string());
    eventually("接收方删掉了收到一半的部分", || {
        !partial.exists()
    })
    .await;
}

#[tokio::test]
async fn sending_without_a_connection_says_so() {
    let device = Device::new();
    let files = Files::new(&device.store, Events::none());
    let source = TempDir::new();
    let file = source.path().join("a.txt");
    write(&file, b"a");
    assert!(matches!(
        files
            .send("00".repeat(32).parse().unwrap(), vec![file])
            .await,
        Err(Error::NotConnected)
    ));
}

#[tokio::test]
async fn an_interrupted_transfer_resumes_where_it_stopped() {
    let (mac_events, mac) = collect();
    let (pc_events, pc) = collect();
    let mut rig = Rig::with(Setup {
        mac_events,
        pc_events,
        ..Setup::default()
    })
    .await;
    let source = TempDir::new();
    let file = source.path().join("发布录屏.mov");
    let content = pattern(48 << 20);
    write(&file, &content);

    send_when_ready(&rig.mac_files, vec![file]).await;
    eventually("开始收了一部分", || {
        pc.lock()
            .unwrap()
            .iter()
            .any(|u| u.state == TransferState::Running && u.done >= 1 << 20)
    })
    .await;
    rig.restart_client();
    eventually("发送方知道断了", || {
        reached(&mac, |s| matches!(s, TransferState::Interrupted))
    })
    .await;

    let target = received(&rig.pc).join("发布录屏.mov");
    // 几十 MB 在较慢的机器上要多等一会儿。
    for _ in 0..1500 {
        if target.is_file() {
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        fs::read(&target).unwrap(),
        content,
        "接着收的内容和原文件一致"
    );
    let resumed =
        pc.lock().unwrap().iter().any(|u| {
            u.state == TransferState::Running && u.done > 0 && u.done < content.len() as u64
        });
    assert!(resumed);
    let starts: Vec<u64> = {
        let seen = pc.lock().unwrap();
        let interrupted = seen
            .iter()
            .position(|u| u.state == TransferState::Interrupted)
            .expect("接收方也报告了中断");
        seen[interrupted..]
            .iter()
            .filter(|u| u.state == TransferState::Running)
            .map(|u| u.done)
            .collect()
    };
    assert!(
        starts.first().is_some_and(|&done| done > 0),
        "重连后从上次的位置开始，而不是从 0：{starts:?}"
    );
}
