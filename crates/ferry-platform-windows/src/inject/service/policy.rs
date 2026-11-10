//! 管理员单独授予的 UAC 设备权限，固定 HKLM 策略；不读取用户配对配置或环境变量。

use ferry_platform::Result;
use ferry_trust::{Fingerprint, InputAuthorization};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, HANDLE};
use windows::Win32::Security::Authorization::{ConvertStringSidToSidW, SE_REGISTRY_KEY};
use windows::Win32::Security::PSID;
use windows::Win32::System::Registry::*;
use windows::core::{HSTRING, PCWSTR};

use super::security::{Descriptor, validate_acl};
use super::util::{failure, os};

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: 本对象独占 RegOpen/CreateKey 返回的句柄，不关闭 HKLM 伪句柄。
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

pub(super) struct Policy {
    key: Key,
    value: String,
    public_key: [u8; 65],
}

impl Policy {
    pub fn load(user: &str, peer: [u8; 32]) -> Result<Option<Self>> {
        let value = value_name(user, Fingerprint::from_bytes(peer))?;
        let Some(key) = open(&["SOFTWARE", "ScreenFerry", "SecureInput"], false, false)? else {
            return Ok(None);
        };
        let Some(public_key) = read(&key, &value)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            key,
            value,
            public_key,
        }))
    }

    pub fn public_key(&self) -> [u8; 65] {
        self.public_key
    }

    pub fn valid(&self) -> Result<bool> {
        Ok(read(&self.key, &self.value)? == Some(self.public_key))
    }
}

pub fn authorize(user: &str, code: &str) -> Result<()> {
    let authorization = InputAuthorization::from_code(code).map_err(|e| failure(&e.to_string()))?;
    let value = value_name(user, authorization.fingerprint)?;
    let key = open(&["SOFTWARE", "ScreenFerry", "SecureInput"], true, true)?
        .ok_or_else(|| failure("无法创建输入授权策略"))?;
    // SAFETY: 管理员才能取得 KEY_SET_VALUE；固定值名是证书指纹，值仅为公开公钥。
    unsafe {
        RegSetValueExW(
            key.0,
            &HSTRING::from(value),
            None,
            REG_BINARY,
            Some(&authorization.public_key),
        )
    }
    .ok()
    .map_err(|e| os("登记安全输入设备授权", e))
}

pub fn revoke(user: &str, peer: &str) -> Result<()> {
    let peer: Fingerprint = peer
        .parse()
        .map_err(|e: ferry_trust::Error| failure(&e.to_string()))?;
    let value = value_name(user, peer)?;
    let Some(key) = open(&["SOFTWARE", "ScreenFerry", "SecureInput"], true, false)? else {
        return Ok(());
    };
    // SAFETY: 固定策略下删除一个指纹，不删除用户配对或其他注册表项。
    let result = unsafe { RegDeleteValueW(key.0, &HSTRING::from(value)) };
    if result == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        result.ok().map_err(|e| os("撤销安全输入设备授权", e))
    }
}

/// 关闭功能或卸载时撤销全部机器级授权；不递归删除，也不删除用户配对。
pub(super) fn clear() -> Result<()> {
    let Some(_policy) = open(&["SOFTWARE", "ScreenFerry", "SecureInput"], true, false)? else {
        return Ok(());
    };
    let parent = open(&["SOFTWARE", "ScreenFerry"], true, false)?
        .ok_or_else(|| failure("授权策略父项已消失"))?;
    // SAFETY: 只删除验证过的固定叶项；RegDeleteKeyEx 不递归、不跟随任意子项。
    // 本策略只保存 SID.指纹 值，不创建子项。未知子项使删除失败，不能扩大删除范围。
    unsafe {
        RegDeleteKeyExW(
            parent.0,
            &HSTRING::from("SecureInput"),
            KEY_WOW64_64KEY.0,
            None,
        )
    }
    .ok()
    .map_err(|e| os("清除安全输入授权", e))
}

fn value_name(user: &str, peer: Fingerprint) -> Result<String> {
    // 严格限制 SID 为一个路径分量，随后交给 Windows 检查 SID 语法。
    if !user.starts_with("S-1-")
        || user.len() > 184
        || !user
            .bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'S' | b'-'))
    {
        return Err(failure("需要完整用户 SID，不能使用账户名或注册表路径"));
    }
    let mut sid = PSID::default();
    // SAFETY: 系统检查 SID 字符串，只保留语法验证结果，随后释放分配。
    unsafe {
        ConvertStringSidToSidW(&HSTRING::from(user), &mut sid)
            .map_err(|e| os("验证授权用户 SID", e))?;
        let _ =
            windows::Win32::Foundation::LocalFree(Some(windows::Win32::Foundation::HLOCAL(sid.0)));
    }
    Ok(format!("{user}.{}", peer.to_hex()))
}

fn open(segments: &[&str], write: bool, create: bool) -> Result<Option<Key>> {
    let mut parent = HKEY_LOCAL_MACHINE;
    let mut keys = Vec::new();
    for &segment in segments {
        let mut key = HKEY::default();
        let rights = KEY_READ | KEY_WOW64_64KEY | if write { KEY_WRITE } else { REG_SAM_FLAGS(0) };
        // SAFETY: 各分量固定或经 SID 校验；OPEN_LINK 打开链接本身，不跟随注册表符号链接。
        let result = unsafe {
            RegOpenKeyExW(
                parent,
                &HSTRING::from(segment),
                Some(REG_OPTION_OPEN_LINK.0),
                rights,
                &mut key,
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            if !create {
                return Ok(None);
            }
            let descriptor =
                Descriptor::from_sddl("O:BAD:P(A;CI;KA;;;SY)(A;CI;KA;;;BA)(A;CI;KR;;;BU)")?;
            let attributes = descriptor.attributes(false);
            // SAFETY: 父项已在上一轮验证不可被普通用户写入；新项创建时即具备保护 ACL。
            unsafe {
                RegCreateKeyExW(
                    parent,
                    &HSTRING::from(segment),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    rights,
                    Some(&attributes),
                    &mut key,
                    None,
                )
            }
            .ok()
            .map_err(|e| os("创建安全输入策略", e))?;
        } else {
            result.ok().map_err(|e| os("打开安全输入策略", e))?;
        }
        let key = Key(key);
        validate_acl(HANDLE(key.0.0), SE_REGISTRY_KEY, 0x500D_0026)?;
        let mut kind = REG_VALUE_TYPE::default();
        // SAFETY: 只查询类型，不读取任意长度数据。链接项一律拒绝。
        let linked = unsafe {
            RegQueryValueExW(
                key.0,
                &HSTRING::from("SymbolicLinkValue"),
                None,
                Some(&mut kind),
                None,
                None,
            )
        };
        if linked.is_ok() && kind == REG_LINK {
            return Err(failure("安全输入策略不能经过注册表链接"));
        }
        parent = key.0;
        keys.push(key);
    }
    Ok(keys.pop())
}

fn read(key: &Key, value: &str) -> Result<Option<[u8; 65]>> {
    let mut public_key = [0; 65];
    let mut kind = REG_VALUE_TYPE::default();
    let mut length = public_key.len() as u32;
    // SAFETY: 固定 65 字节缓冲区；更大、截断或非 binary 值都拒绝，不根据策略分配内存。
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            &HSTRING::from(value),
            None,
            Some(&mut kind),
            Some(public_key.as_mut_ptr()),
            Some(&mut length),
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    result.ok().map_err(|e| os("读取安全输入授权", e))?;
    if kind != REG_BINARY || length != 65 || public_key[0] != 4 {
        return Err(failure("安全输入策略中的公钥无效"));
    }
    Ok(Some(public_key))
}
