//! 更新清单 `update.json`：格式、签名校验、解析成 [`Release`]。

use std::collections::BTreeMap;

use semver::Version;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};

/// 现在的清单格式。程序不认识更新的格式时不更新，免得误读。
pub const SCHEMA: u32 = 1;

/// 清单的原样：CI 写它，程序读它。时间是 RFC 3339 字符串，方便人看。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    /// 新版本号，例如 `0.2.0`。
    pub version: String,
    /// 发布时间。
    pub published: String,
    /// 过了这个时间，这份清单就不再可信：可能有人一直给出旧清单，挡住新版本。
    pub expires: String,
    /// 新版本的握手格式标记（`ferry_proto::WIRE_FORMAT`，固定为 1）。字段名沿用已发布的清单格式。
    /// 和本机不同时，两台要一起更新才能互连；正常情况下永远相同。
    pub protocol: u16,
    #[serde(default)]
    pub notes: Notes,
    /// 各平台的更新包，键是 `macos-universal`、`windows-x86_64` 这样的平台名。
    pub assets: BTreeMap<String, Asset>,
}

/// 更新说明，每行一条。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notes {
    #[serde(default)]
    pub zh: String,
    #[serde(default)]
    pub en: String,
}

/// 一个平台的更新包。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub url: String,
    pub size: u64,
    /// 小写十六进制。
    pub sha256: String,
}

impl Asset {
    /// 下载下来的文件是不是这一个：大小和 SHA-256 都要对得上。
    pub fn matches(&self, size: u64, sha256: &str) -> bool {
        self.size == size && self.sha256.eq_ignore_ascii_case(sha256)
    }

    /// 下载地址里的文件名。
    pub fn file_name(&self) -> &str {
        let path = self.url.split(['?', '#']).next().unwrap_or("");
        path.rsplit('/').next().unwrap_or("")
    }
}

/// 校验过签名、解析好的清单。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// Unix 秒。
    pub published: i64,
    /// Unix 秒。
    pub expires: i64,
    pub protocol: u16,
    pub notes: Notes,
    pub assets: BTreeMap<String, Asset>,
}

impl Release {
    /// 校验 `manifest`（清单的原始字节）的 minisign 签名，再解析。`keys` 里任何一把对得上就行。
    pub fn verify(manifest: &[u8], signature: &str, keys: &[&str]) -> Result<Self> {
        if keys.is_empty() {
            return Err(Error::NoKeys);
        }
        let signature = minisign_verify::Signature::decode(signature)
            .map_err(|e| Error::BadSignature(e.to_string()))?;
        let mut trusted = false;
        for key in keys {
            let key = minisign_verify::PublicKey::from_base64(key)
                .map_err(|e| Error::BadKey(e.to_string()))?;
            if key.verify(manifest, &signature, false).is_ok() {
                trusted = true;
                break;
            }
        }
        if !trusted {
            return Err(Error::Untrusted);
        }
        let manifest: Manifest =
            serde_json::from_slice(manifest).map_err(|e| Error::BadManifest(e.to_string()))?;
        Self::from_manifest(manifest)
    }

    /// 只解析，不校验签名。程序里只能用 [`Release::verify`]；这个给发布工具检查自己写的清单。
    pub fn from_manifest(manifest: Manifest) -> Result<Self> {
        if manifest.schema != SCHEMA {
            return Err(Error::UnknownSchema(manifest.schema));
        }
        let version = Version::parse(&manifest.version)
            .map_err(|e| Error::BadManifest(format!("版本号 {}：{e}", manifest.version)))?;
        Ok(Self {
            version,
            published: parse_time(&manifest.published)?,
            expires: parse_time(&manifest.expires)?,
            protocol: manifest.protocol,
            notes: manifest.notes,
            assets: manifest.assets,
        })
    }

    /// 这个平台的更新包；没有时要手动安装。
    pub fn asset(&self, platform: &str) -> Option<&Asset> {
        self.assets.get(platform)
    }
}

/// RFC 3339 时间换成 Unix 秒。
fn parse_time(text: &str) -> Result<i64> {
    OffsetDateTime::parse(text, &Rfc3339)
        .map(OffsetDateTime::unix_timestamp)
        .map_err(|e| Error::BadManifest(format!("时间 {text}：{e}")))
}

impl Manifest {
    /// Unix 秒写成 RFC 3339（UTC），发布工具写清单时用。
    pub fn format_time(unix: i64) -> String {
        OffsetDateTime::from_unix_timestamp(unix)
            .ok()
            .and_then(|t| t.format(&Rfc3339).ok())
            .unwrap_or_default()
    }
}
