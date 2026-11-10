//! 管道身份与服务文件权限。服务程序的位置只从本程序自身或 SCM 的登记得到，不接受客户端提供的路径。
//!
//! 服务程序固定放在系统盘受保护的目录里，和 ScreenFerry 装在哪个盘无关：
//! 后台程序可以在任何位置，它不属于受信任的部分——特权输入只认管理员授权过的远端设备签名。

use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use ferry_platform::Result;
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows::Win32::Security::*;
use windows::Win32::Storage::FileSystem::{
    CreateDirectoryW, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};
use windows::Win32::System::RemoteDesktop::{
    ProcessIdToSessionId, WTSGetActiveConsoleSessionId, WTSQueryUserToken,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW,
};
use windows::core::{BOOL, HSTRING, PWSTR};

use super::SERVICE_EXE;
use super::util::{Handle, failure, os};

pub(super) struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    pub fn raw(&self) -> PSECURITY_DESCRIPTOR {
        self.0
    }
    pub fn from_sddl(sddl: &str) -> Result<Self> {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: 有效的 SDDL 字符串，Windows 分配的描述符由 Drop 用 LocalFree 释放。
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &HSTRING::from(sddl),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|e| os("创建输入服务权限", e))?;
        Ok(Self(descriptor))
    }

    pub fn attributes(&self, inherit: bool) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0.0,
            bInheritHandle: BOOL(i32::from(inherit)),
        }
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: 描述符由 Windows 的 LocalAlloc 分配，只有本对象释放。
        let _ = unsafe { LocalFree(Some(HLOCAL(self.0.0))) };
    }
}

pub(super) struct Installed {
    pub agent: PathBuf,
    /// 保留打开的句柄，验证后不允许目录或映像被重命名/替换。
    _files: Vec<Handle>,
}

impl Installed {
    /// 服务和代理用：服务程序就是本程序自己。
    pub fn load() -> Result<Self> {
        Self::at(&std::env::current_exe().map_err(|e| failure(&e.to_string()))?)
    }

    /// 后台用：服务程序在 SCM 登记的位置。登记只有管理员能改，后台自己在哪个盘都可以。
    pub fn registered() -> Result<Self> {
        Self::at(&super::controller::image()?)
    }

    fn at(agent: &Path) -> Result<Self> {
        if !agent
            .file_name()
            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(SERVICE_EXE))
        {
            return Err(failure("不是 ScreenFerry 的输入服务程序"));
        }
        let directory = agent.parent().ok_or_else(|| failure("没有服务目录"))?;
        let mut files = Vec::new();
        for ancestor in directory.ancestors() {
            files.push(protected(ancestor, ancestor == directory)?);
        }
        files.push(protected(agent, true)?);
        // SYSTEM 代理只从服务目录和 System32 加载 DLL；目录中不能留下用户可改的文件。
        for entry in directory.read_dir().map_err(|e| failure(&e.to_string()))? {
            let path = entry.map_err(|e| failure(&e.to_string()))?.path();
            if path.is_file() {
                files.push(protected(&path, true)?);
            }
        }
        Ok(Self {
            agent: canonical(agent)?,
            _files: files,
        })
    }
}

/// 由安装包中临时解出的可信工具在复制文件前调用；不执行目标目录中的程序。
pub(super) fn prepare_directory(directory: &Path) -> Result<()> {
    use std::path::{Component, Prefix};
    if !directory.is_absolute()
        || !matches!(directory.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        || !directory
            .file_name()
            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("ScreenFerry"))
        || directory
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(failure("输入服务需要本机受保护的 ScreenFerry 目录"));
    }
    let mut guards = Vec::new();
    for parent in directory.parent().into_iter().flat_map(Path::ancestors) {
        guards.push(protected(parent, false)?);
    }
    if !directory.exists() {
        let descriptor =
            Descriptor::from_sddl("O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;BU)")?;
        let attributes = descriptor.attributes(false);
        // SAFETY: 父目录已验证并保持打开。目录在创建时就有保护 ACL，不存在先创建后收紧的窗口。
        unsafe { CreateDirectoryW(&HSTRING::from(directory.as_os_str()), Some(&attributes)) }
            .map_err(|e| os("创建受保护的服务目录", e))?;
    }
    guards.push(protected(directory, true)?);
    for entry in directory.read_dir().map_err(|e| failure(&e.to_string()))? {
        let path = entry.map_err(|e| failure(&e.to_string()))?.path();
        if path.is_file() {
            guards.push(protected(&path, true)?);
        }
    }
    Ok(())
}

/// 安装器已取得管理员权限后，只收紧服务目录和服务程序的 ACL。
pub(super) fn harden_installation() -> Result<()> {
    let current = std::env::current_exe().map_err(|e| failure(&e.to_string()))?;
    let directory = current.parent().ok_or_else(|| failure("没有服务目录"))?;
    // 先验证所有上级；不能把服务装在用户能改名、替换的目录里。
    for ancestor in directory.parent().into_iter().flat_map(Path::ancestors) {
        protected(ancestor, false)?;
    }
    for (path, directory) in [
        (directory.to_owned(), true),
        (directory.join(SERVICE_EXE), false),
    ] {
        let flags = if directory { "OICI" } else { "" };
        let descriptor = Descriptor::from_sddl(&format!(
            "O:BAD:P(A;{flags};FA;;;SY)(A;{flags};FA;;;BA)(A;{flags};GRGX;;;BU)"
        ))?;
        let mut owner = PSID::default();
        let mut defaulted = BOOL(0);
        let mut present = BOOL(0);
        let mut acl = std::ptr::null_mut();
        // SAFETY: 描述符保持存活，输出指针有效。仅修改固定安装目录和固定文件的 owner/DACL。
        unsafe {
            GetSecurityDescriptorOwner(descriptor.0, &mut owner, &mut defaulted)
                .map_err(|e| os("读取安装 owner", e))?;
            GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut acl, &mut defaulted)
                .map_err(|e| os("读取安装 DACL", e))?;
            SetNamedSecurityInfoW(
                &HSTRING::from(path.as_os_str()),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION
                    | DACL_SECURITY_INFORMATION
                    | PROTECTED_DACL_SECURITY_INFORMATION,
                Some(owner),
                None,
                Some(acl),
                None,
            )
            .ok()
            .map_err(|e| os("保护输入服务文件", e))?;
        }
    }
    Installed::load().map(drop)
}

fn canonical(path: &Path) -> Result<PathBuf> {
    path.canonicalize().map_err(|e| failure(&e.to_string()))
}

fn protected(path: &Path, strict: bool) -> Result<Handle> {
    let metadata = path
        .symlink_metadata()
        .map_err(|e| failure(&e.to_string()))?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(failure("输入服务路径不能经过重解析点"));
    }
    // SAFETY: 用只读权限打开路径本身而非重解析目标；不共享删除，保证后续校验对应同一对象。
    let handle = unsafe {
        CreateFileW(
            &HSTRING::from(path.as_os_str()),
            0x0002_0000,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|e| os("打开受保护的安装路径", e))?;
    let handle = Handle::new(handle)?;
    validate_acl(
        handle.0,
        SE_FILE_OBJECT,
        0x500D_0152 | if strict { 0x4 } else { 0 },
    )?;
    Ok(handle)
}

/// 校验同一内核对象的 owner 与所有允许 ACE；注册表策略和文件共用保守规则。
pub(super) fn validate_acl(
    handle: HANDLE,
    kind: Authorization::SE_OBJECT_TYPE,
    dangerous: u32,
) -> Result<()> {
    let mut owner = PSID::default();
    let mut acl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: 系统返回与 handle 对应的有效描述符，返回缓冲区由 Descriptor 释放。
    unsafe {
        GetSecurityInfo(
            handle,
            kind,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            Some(&mut acl),
            None,
            Some(&mut descriptor),
        )
    }
    .ok()
    .map_err(|e| os("查询安装路径权限", e))?;
    let _descriptor = Descriptor(descriptor);
    if !trusted(owner)? || acl.is_null() {
        return Err(failure("输入服务路径必须由 SYSTEM 或管理员保护"));
    }
    // 禁止非可信主体修改文件、替换目录或改 ACL；上级目录允许创建无关子目录，但不允许删除已有子项。
    // SAFETY: acl 和每条 ACE 都由 GetSecurityInfo 返回，描述符在整个遍历期间有效。
    unsafe {
        for index in 0..u32::from((*acl).AceCount) {
            let mut ace = std::ptr::null_mut();
            GetAce(acl, index, &mut ace).map_err(|e| os("读取安装 ACL", e))?;
            let header = &*ace.cast::<ACE_HEADER>();
            if u32::from(header.AceFlags) & INHERIT_ONLY_ACE.0 != 0 {
                continue;
            }
            // ACCESS_ALLOWED_ACE_TYPE；其他允许类型（对象、回调等）保守拒绝。
            if header.AceType == 0 {
                let allowed = &*ace.cast::<ACCESS_ALLOWED_ACE>();
                let sid = PSID((&allowed.SidStart as *const u32).cast_mut().cast());
                if allowed.Mask & dangerous != 0 && !trusted(sid)? {
                    return Err(failure("输入服务安装路径对非管理员可写"));
                }
            } else if header.AceType != 1 && header.AceType != 2 && header.AceType != 17 {
                return Err(failure("输入服务安装路径含不支持的访问规则"));
            }
        }
    }
    Ok(())
}

fn trusted(sid: PSID) -> Result<bool> {
    let sid = sid_text(sid)?;
    Ok(matches!(
        sid.as_str(),
        "S-1-5-18"
            | "S-1-5-32-544"
            | "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
    ))
}

fn sid_text(sid: PSID) -> Result<String> {
    let mut text = PWSTR::null();
    // SAFETY: sid 来自系统描述符或令牌。输出由 Windows 分配，转换后立即释放。
    unsafe {
        ConvertSidToStringSidW(sid, &mut text).map_err(|e| os("读取 SID", e))?;
        let result = text.to_string().map_err(|e| failure(&e.to_string()));
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        result
    }
}

pub(super) fn token_sid(token: HANDLE) -> Result<String> {
    let mut needed = 0;
    // SAFETY: 首次查询只取长度；Vec<usize> 提供 TOKEN_USER/SID 所需对齐。
    unsafe {
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        if needed < size_of::<TOKEN_USER>() as u32 || needed > 65536 {
            return Err(failure("非法令牌长度"));
        }
        let mut buffer = vec![0_usize; (needed as usize).div_ceil(size_of::<usize>())];
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
        .map_err(|e| os("读取进程令牌", e))?;
        sid_text((*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid)
    }
}

pub(super) fn process_sid(process: HANDLE) -> Result<String> {
    let mut token = HANDLE::default();
    // SAFETY: 有效进程句柄；只申请 TOKEN_QUERY，返回的令牌由 Handle 释放。
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }
        .map_err(|e| os("查询进程身份", e))?;
    token_sid(Handle::new(token)?.0)
}

pub(super) fn require_system() -> Result<()> {
    // SAFETY: 当前进程伪句柄只借用，不关闭。
    if process_sid(unsafe { GetCurrentProcess() })? != "S-1-5-18" {
        return Err(failure("输入代理必须由 Windows SYSTEM 服务启动"));
    }
    Ok(())
}

/// 会话用户查询完成后，代理永久移除除目录遍历外的全部令牌特权。
/// 服务自己的令牌不受影响；代理不能再启用调试、备份、恢复或 TCB 等特权。
pub(super) fn drop_agent_privileges() -> Result<()> {
    let mut token = HANDLE::default();
    // SAFETY: 只修改当前代理自己的主令牌。缓冲区由内核返回并按 TOKEN_PRIVILEGES 对齐。
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES,
            &mut token,
        )
        .map_err(|e| os("打开代理权限", e))?;
        let token = Handle::new(token)?;
        let mut keep = windows::Win32::Foundation::LUID::default();
        LookupPrivilegeValueW(
            windows::core::PCWSTR::null(),
            &HSTRING::from("SeChangeNotifyPrivilege"),
            &mut keep,
        )
        .map_err(|e| os("查询遍历权限", e))?;
        let mut needed = 0;
        let _ = GetTokenInformation(token.0, TokenPrivileges, None, 0, &mut needed);
        if needed < size_of::<TOKEN_PRIVILEGES>() as u32 || needed > 65536 {
            return Err(failure("非法代理权限长度"));
        }
        let mut buffer = vec![0_usize; (needed as usize).div_ceil(size_of::<usize>())];
        GetTokenInformation(
            token.0,
            TokenPrivileges,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
        .map_err(|e| os("查询代理权限", e))?;
        let privileges = buffer.as_mut_ptr().cast::<TOKEN_PRIVILEGES>();
        let count = (*privileges).PrivilegeCount as usize;
        if core::mem::offset_of!(TOKEN_PRIVILEGES, Privileges)
            + count * size_of::<LUID_AND_ATTRIBUTES>()
            > needed as usize
        {
            return Err(failure("非法代理权限数量"));
        }
        for privilege in
            std::slice::from_raw_parts_mut((*privileges).Privileges.as_mut_ptr(), count)
        {
            if privilege.Luid != keep {
                privilege.Attributes = SE_PRIVILEGE_REMOVED;
            }
        }
        AdjustTokenPrivileges(token.0, false, Some(privileges), 0, None, None)
            .map_err(|e| os("移除代理多余特权", e))?;
    }
    Ok(())
}

pub(super) fn session_of(pid: u32) -> Result<u32> {
    let mut session = 0;
    // SAFETY: 只读查询 PID 的会话，输出指向有效的 u32。
    unsafe { ProcessIdToSessionId(pid, &mut session) }.map_err(|e| os("查询登录会话", e))?;
    Ok(session)
}

pub(super) fn current_session() -> Result<u32> {
    // SAFETY: 当前进程 ID 的只读查询。
    session_of(unsafe { GetCurrentProcessId() })
}

pub(super) fn active_session() -> u32 {
    // SAFETY: 无参数只读查询；没有控制台会话时返回 u32::MAX。
    unsafe { WTSGetActiveConsoleSessionId() }
}

pub(super) fn session_user(session: u32) -> Result<String> {
    session_identity(session).map(|(user, _)| user)
}

/// SID 相同不代表同一次登录；AuthenticationId 防止会话号被重用时沿用旧代理。
pub(super) fn session_identity(session: u32) -> Result<(String, u64)> {
    let mut token = HANDLE::default();
    // SAFETY: 仅 SYSTEM 调用，取得当前会话用户的只读查询令牌，随后关闭。
    unsafe { WTSQueryUserToken(session, &mut token) }.map_err(|e| os("查询登录用户", e))?;
    let token = Handle::new(token)?;
    let user = token_sid(token.0)?;
    let mut statistics = TOKEN_STATISTICS::default();
    let mut needed = 0;
    // SAFETY: 匹配 TokenStatistics 的完整输出结构，令牌在调用期间有效。
    unsafe {
        GetTokenInformation(
            token.0,
            TokenStatistics,
            Some((&mut statistics as *mut TOKEN_STATISTICS).cast()),
            size_of::<TOKEN_STATISTICS>() as u32,
            &mut needed,
        )
    }
    .map_err(|e| os("查询登录身份", e))?;
    let logon = (u64::from(statistics.AuthenticationId.HighPart as u32) << 32)
        | u64::from(statistics.AuthenticationId.LowPart);
    Ok((user, logon))
}

/// 后台核验管道另一头是登记过的 SYSTEM 服务程序，防止别的程序抢先建同名管道冒充代理。
pub(super) fn verify_server(pipe: HANDLE, installed: &Installed, session: u32) -> Result<Handle> {
    let mut pid = 0;
    // SAFETY: 从本对象的管道句柄查询内核记录的对端 PID，不信任请求中的身份字段。
    unsafe { GetNamedPipeServerProcessId(pipe, &mut pid) }
        .map_err(|e| os("核验输入代理对端", e))?;
    verified_process(
        pid,
        Some(&installed.agent),
        session,
        "S-1-5-18",
        Default::default(),
    )
}

/// 代理核验管道另一头是同一登录会话里的同一个用户。
/// 不核对它是哪个程序：后台可以装在任何盘，那里的文件不受保护；特权输入的把关在远端设备签名和管理员授权。
pub(super) fn verify_client(pipe: HANDLE, session: u32, user: &str) -> Result<Handle> {
    let mut pid = 0;
    // SAFETY: 从本对象的管道句柄查询内核记录的对端 PID，不信任请求中的身份字段。
    unsafe { GetNamedPipeClientProcessId(pipe, &mut pid) }
        .map_err(|e| os("核验输入代理对端", e))?;
    verified_process(pid, None, session, user, Default::default())
}

pub(super) fn service_parent(pid: u32, installed: &Installed) -> Result<Handle> {
    verified_process(
        pid,
        Some(&installed.agent),
        0,
        "S-1-5-18",
        windows::Win32::System::Threading::PROCESS_DUP_HANDLE,
    )
}

fn verified_process(
    pid: u32,
    expected: Option<&Path>,
    session: u32,
    user: &str,
    extra: windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS,
) -> Result<Handle> {
    let process = Handle::new(
        unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | extra,
                false,
                pid,
            )
        }
        .map_err(|e| os("核验输入代理进程", e))?,
    )?;
    if session_of(pid)? != session {
        return Err(failure("输入代理会话不匹配"));
    }
    if process_sid(process.0)? != user {
        return Err(failure("拒绝未经授权的输入代理进程"));
    }
    if let Some(expected) = expected {
        let mut image = vec![0_u16; 32768];
        let mut len = image.len() as u32;
        // SAFETY: 只读查询映像名称，长度不超过实际缓冲区。
        unsafe {
            QueryFullProcessImageNameW(
                process.0,
                PROCESS_NAME_WIN32,
                PWSTR(image.as_mut_ptr()),
                &mut len,
            )
        }
        .map_err(|e| os("核验输入代理映像", e))?;
        let image = canonical(Path::new(&String::from_utf16_lossy(&image[..len as usize])))?;
        if !image
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy())
        {
            return Err(failure("拒绝未经授权的输入代理进程"));
        }
    }
    Ok(process)
}
