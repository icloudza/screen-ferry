//! 服务只启动固定程序；不继承句柄、挂起创建和 Job 绑定先于代理执行。

use ferry_platform::Result;
use windows::Win32::Foundation::{
    ERROR_NOT_ALL_ASSIGNED, ERROR_SUCCESS, GetLastError, HANDLE, SetLastError,
};
use windows::Win32::Security::*;
use windows::Win32::System::JobObjects::*;
use windows::Win32::System::Threading::*;
use windows::core::{HSTRING, PCWSTR, PWSTR};

use super::security::{Descriptor, Installed};
use super::util::{Handle, failure, os};

pub(super) struct AgentProcess {
    job: Handle,
    process: Handle,
    stop: Handle,
}

impl AgentProcess {
    pub fn launch(installed: &Installed, session: u32, user: &str) -> Result<Self> {
        for privilege in [
            "SeTcbPrivilege",
            "SeAssignPrimaryTokenPrivilege",
            "SeIncreaseQuotaPrivilege",
        ] {
            enable(privilege)?;
        }
        let event_acl = Descriptor::from_sddl("D:P(A;;GA;;;SY)")?;
        let event_attributes = event_acl.attributes(false);
        // SAFETY: 未命名、不可继承的事件；代理验证父服务后只复制 SYNCHRONIZE 权限。
        let stop = Handle::new(
            unsafe { CreateEventW(Some(&event_attributes), true, false, PCWSTR::null()) }
                .map_err(|e| os("创建代理停止事件", e))?,
        )?;
        let job = Handle::new(
            unsafe { CreateJobObjectW(None, PCWSTR::null()) }
                .map_err(|e| os("创建输入代理 Job", e))?,
        )?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        // SAFETY: 有效 Job 和匹配的完整结构；服务退出或崩溃时代理不能继续存活。
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of_val(&limits) as u32,
            )
        }
        .map_err(|e| os("绑定输入代理生命周期", e))?;

        // 用户只能查询代理令牌以验证 SYSTEM 身份，不能复制、修改或冒充令牌。
        let token_acl =
            Descriptor::from_sddl(&format!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x0008;;;{user})"))?;
        let token_attributes = token_acl.attributes(false);
        let token = session_token(session, &token_attributes)?;
        let process_acl = Descriptor::from_sddl(&format!(
            "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x00101000;;;{user})"
        ))?;
        let process_attributes = process_acl.attributes(false);
        let mut startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let desktop = HSTRING::from(r"winsta0\Default");
        startup.lpDesktop = PWSTR(desktop.as_ptr().cast_mut());
        let executable = HSTRING::from(installed.agent.as_os_str());
        let directory = HSTRING::from(installed.agent.parent().expect("固定安装路径").as_os_str());
        let command = format!(
            "\"{}\" agent {} {}",
            installed.agent.display(),
            unsafe { GetCurrentProcessId() },
            stop.0.0 as usize
        );
        let mut command: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: 固定可执行文件和参数；令牌是本服务自己的 SYSTEM 主令牌，只有会话号变化。
        // CREATE_SUSPENDED 保证绑定 Job 成功前，子进程不执行任何代码。
        unsafe {
            CreateProcessAsUserW(
                Some(token.0),
                &executable,
                Some(PWSTR(command.as_mut_ptr())),
                Some(&process_attributes),
                None,
                false,
                CREATE_SUSPENDED | CREATE_NO_WINDOW,
                None,
                &directory,
                &startup,
                &mut info,
            )
        }
        .map_err(|e| os("启动会话输入代理", e))?;
        let process = Handle::new(info.hProcess)?;
        let thread = Handle::new(info.hThread)?;
        // SAFETY: 子进程仍挂起；失败立即终止，不留没有 Job 监管的 SYSTEM 进程。
        unsafe {
            if let Err(e) = AssignProcessToJobObject(job.0, process.0) {
                let _ = TerminateProcess(process.0, 1);
                return Err(os("监管会话输入代理", e));
            }
            if ResumeThread(thread.0) == u32::MAX {
                return Err(failure("无法恢复输入代理线程"));
            }
        }
        Ok(Self { job, process, stop })
    }

    pub fn exited(&self) -> bool {
        self.process.signalled()
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        // SAFETY: 先给代理机会释放按键和光标；只有超过清理时限才终止整个 Job。
        unsafe {
            let _ = SetEvent(self.stop.0);
            if !self.process.wait(2500) {
                let _ = TerminateJobObject(self.job.0, 1);
            }
        }
    }
}

fn session_token(session: u32, attributes: &SECURITY_ATTRIBUTES) -> Result<Handle> {
    let mut source = HANDLE::default();
    let mut copied = HANDLE::default();
    // SAFETY: 仅 SYSTEM 服务可走到此处。复制自身令牌，不借用其他进程；所有句柄有唯一所有者。
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_DUPLICATE,
            &mut source,
        )
        .map_err(|e| os("查询服务令牌", e))?;
        let source = Handle::new(source)?;
        DuplicateTokenEx(
            source.0,
            TOKEN_ALL_ACCESS,
            Some(attributes),
            SecurityImpersonation,
            TokenPrimary,
            &mut copied,
        )
        .map_err(|e| os("创建输入代理令牌", e))?;
        let copied = Handle::new(copied)?;
        SetTokenInformation(
            copied.0,
            TokenSessionId,
            (&session as *const u32).cast(),
            size_of::<u32>() as u32,
        )
        .map_err(|e| os("设置输入代理会话", e))?;
        Ok(copied)
    }
}

fn enable(name: &str) -> Result<()> {
    let mut token = HANDLE::default();
    // SAFETY: 仅开启本服务令牌中已经拥有的指定权限；不改变用户进程或系统策略。
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .map_err(|e| os("打开服务权限", e))?;
        let token = Handle::new(token)?;
        let mut privilege = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            ..Default::default()
        };
        LookupPrivilegeValueW(
            PCWSTR::null(),
            &HSTRING::from(name),
            &mut privilege.Privileges[0].Luid,
        )
        .map_err(|e| os("查询服务权限", e))?;
        privilege.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        SetLastError(ERROR_SUCCESS);
        AdjustTokenPrivileges(token.0, false, Some(&privilege), 0, None, None)
            .map_err(|e| os("启用服务权限", e))?;
        if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
            return Err(failure("SYSTEM 服务缺少输入代理所需权限"));
        }
    }
    Ok(())
}
