//! mDNS 发现：服务端在局域网里广播自己，客户端不填地址也能找到它。
//!
//! 广播内容（TXT 记录）：`fp` 证书指纹、`name` 设备名、`v` 格式标记（固定为 1，见 `ferry_proto::WIRE_FORMAT`）。
//! 这些内容谁都能伪造，只用来找到地址；连上之后仍以 TLS 握手中的证书指纹为准。

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};

use ferry_proto::WIRE_FORMAT;
use ferry_trust::{Fingerprint, PeerList};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::error::{Error, Result};

/// DNS-SD 服务类型。
pub const SERVICE_TYPE: &str = "_screenferry._tcp.local.";

const TXT_FINGERPRINT: &str = "fp";
const TXT_NAME: &str = "name";
const TXT_VERSION: &str = "v";

/// 局域网里发现的一台服务端。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discovered {
    pub name: String,
    /// 对方自称的指纹，连上后须与证书核对。
    pub fingerprint: Fingerprint,
    /// 可尝试的地址，越靠前越可能连通。
    pub addrs: Vec<SocketAddr>,
}

/// 在局域网里广播本机，丢弃即停止广播。
pub struct Advertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Advertiser {
    pub fn start(name: &str, fingerprint: Fingerprint, port: u16) -> Result<Self> {
        let daemon = ServiceDaemon::new().map_err(mdns)?;
        // 实例名在局域网内须唯一，同名设备靠指纹区分。
        let instance = format!("{name} ({})", &fingerprint.short()[..9]);
        let host = format!("screenferry-{}.local.", &fingerprint.to_hex()[..12]);
        let properties = [
            (TXT_FINGERPRINT, fingerprint.to_hex()),
            (TXT_NAME, name.to_owned()),
            (TXT_VERSION, WIRE_FORMAT.to_string()),
        ];
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, &properties[..])
            .map_err(mdns)?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_owned();
        daemon.register(info).map_err(mdns)?;
        Ok(Self { daemon, fullname })
    }
}

impl Drop for Advertiser {
    fn drop(&mut self) {
        // 尽量通知局域网本机已下线；失败也无妨，记录会自然过期。
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// 在后台持续收集局域网里的服务端，丢弃即停止。
pub struct Browser {
    daemon: ServiceDaemon,
    found: Arc<Mutex<BTreeMap<String, Discovered>>>,
}

impl Browser {
    pub fn start() -> Result<Self> {
        let daemon = ServiceDaemon::new().map_err(mdns)?;
        let events = daemon.browse(SERVICE_TYPE).map_err(mdns)?;
        let found = Arc::new(Mutex::new(BTreeMap::new()));
        let shared = found.clone();
        // mdns-sd 自带后台线程，这里再用一个线程把事件整理成列表；daemon 关闭后通道断开，线程随之结束。
        std::thread::Builder::new()
            .name("mdns-browse".into())
            .spawn(move || {
                while let Ok(event) = events.recv() {
                    let mut found = lock(&shared);
                    match event {
                        ServiceEvent::ServiceResolved(info) => match parse(&info) {
                            Some(server) => {
                                found.insert(info.fullname.clone(), server);
                            }
                            None => {
                                found.remove(&info.fullname);
                            }
                        },
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            found.remove(&fullname);
                        }
                        _ => {}
                    }
                }
            })?;
        Ok(Self { daemon, found })
    }

    /// 目前已知的服务端，按实例名排序。
    pub fn found(&self) -> Vec<Discovered> {
        lock(&self.found).values().cloned().collect()
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// 从 mDNS 记录中取出服务端信息。格式标记不同、指纹缺失或没有 IPv4 地址的忽略。
fn parse(info: &mdns_sd::ResolvedService) -> Option<Discovered> {
    if info.get_property_val_str(TXT_VERSION)? != WIRE_FORMAT.to_string() {
        return None;
    }
    let fingerprint = info.get_property_val_str(TXT_FINGERPRINT)?.parse().ok()?;
    let name = info
        .get_property_val_str(TXT_NAME)
        .unwrap_or(&info.fullname)
        .to_owned();
    let ips = info.get_addresses().iter().map(|ip| ip.to_ip_addr());
    let addrs = order(ips)
        .into_iter()
        .map(|ip| SocketAddr::new(ip, info.get_port()))
        .collect::<Vec<_>>();
    (!addrs.is_empty()).then_some(Discovered {
        name,
        fingerprint,
        addrs,
    })
}

/// 只取 IPv4（IPv6 链路本地地址还需要网卡编号），常规地址在前，链路本地和回环在后。
fn order(ips: impl Iterator<Item = IpAddr>) -> Vec<IpAddr> {
    let mut v4: Vec<_> = ips
        .filter_map(|ip| match ip {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })
        .collect();
    v4.sort_by_key(|ip| (ip.is_loopback(), ip.is_link_local(), *ip));
    v4.dedup();
    v4.into_iter().map(IpAddr::V4).collect()
}

/// 客户端该连哪一台。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lookup {
    /// 已配对过的服务端。
    Trusted(Discovered),
    /// 本机还没配对过任何设备，局域网里恰好只有这一台服务端。
    Unpaired(Discovered),
    /// 没有可以直接连的服务端；`others` 是局域网里其他未配对的服务端。
    Nothing { others: Vec<Discovered> },
}

/// 从发现的服务端中挑一台：优先已配对的；从未配对过时，只有一台才自动选它。
///
/// 已经配对过别的设备时，不会自动去连陌生的服务端，要配新设备请指定地址。
pub fn pick(found: &[Discovered], peers: &PeerList) -> Lookup {
    if let Some(server) = found.iter().find(|s| peers.contains(&s.fingerprint)) {
        return Lookup::Trusted(server.clone());
    }
    match found {
        [only] if peers.is_empty() => Lookup::Unpaired(only.clone()),
        others => Lookup::Nothing {
            others: others.to_vec(),
        },
    }
}

fn mdns(e: mdns_sd::Error) -> Error {
    Error::Discovery(e.to_string())
}
