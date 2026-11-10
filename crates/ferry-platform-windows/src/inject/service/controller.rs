//! SCM 生命周期。服务从自身位置派生唯一代理程序，不接受任何用户命令行或配置路径。

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use ferry_platform::Result;
use windows::Win32::Foundation::{
    ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST, ERROR_SERVICE_EXISTS,
    ERROR_SERVICE_NOT_ACTIVE,
};
use windows::Win32::Security::DACL_SECURITY_INFORMATION;
use windows::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_SEARCH_APPLICATION_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
};
use windows::Win32::System::Services::*;
use windows::core::{HSTRING, PCWSTR, PWSTR};

use super::NAME;
use super::process::AgentProcess;
use super::security::{self, Descriptor, Installed};
use super::util::{failure, os};

static STOP: AtomicBool = AtomicBool::new(false);

struct ServiceHandle(SC_HANDLE);
impl Drop for ServiceHandle {
    fn drop(&mut self) {
        // SAFETY: 只关闭本对象独占的 SCM/服务句柄。
        let _ = unsafe { CloseServiceHandle(self.0) };
    }
}

fn manager(access: u32) -> Result<ServiceHandle> {
    // SAFETY: 本机 SCM，不指定远程机器或数据库。
    unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), access) }
        .map(ServiceHandle)
        .map_err(|e| os("打开 Windows 服务管理器", e))
}

pub(super) fn running() -> bool {
    let Ok(manager) = manager(SC_MANAGER_CONNECT) else {
        return false;
    };
    // SAFETY: 只读查询固定服务。
    unsafe {
        let Ok(service) =
            OpenServiceW(manager.0, &HSTRING::from(NAME), SERVICE_QUERY_STATUS).map(ServiceHandle)
        else {
            return false;
        };
        let mut status = SERVICE_STATUS::default();
        QueryServiceStatus(service.0, &mut status).is_ok()
            && matches!(
                status.dwCurrentState,
                SERVICE_RUNNING | SERVICE_START_PENDING
            )
    }
}

pub fn install() -> Result<()> {
    let manager = manager(SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE)?;
    security::harden_installation()?;
    let installed = Installed::load()?;
    let command = HSTRING::from(format!("\"{}\" run", installed.agent.display()));
    let name = HSTRING::from(NAME);
    // SAFETY: 固定服务、固定受保护程序，默认账户 LocalSystem，无任意依赖或密码参数。
    let service = unsafe {
        match CreateServiceW(
            manager.0,
            &name,
            &HSTRING::from("ScreenFerry Secure Input"),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_AUTO_START,
            SERVICE_ERROR_NORMAL,
            &command,
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        ) {
            Ok(handle) => ServiceHandle(handle),
            Err(e) if e.code() == ERROR_SERVICE_EXISTS.to_hresult() => {
                let service = ServiceHandle(
                    OpenServiceW(manager.0, &name, SERVICE_ALL_ACCESS)
                        .map_err(|e| os("更新输入服务", e))?,
                );
                ChangeServiceConfigW(
                    service.0,
                    SERVICE_WIN32_OWN_PROCESS,
                    SERVICE_AUTO_START,
                    SERVICE_ERROR_NORMAL,
                    &command,
                    PCWSTR::null(),
                    None,
                    PCWSTR::null(),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    PCWSTR::null(),
                )
                .map_err(|e| os("更新输入服务配置", e))?;
                service
            }
            Err(e) => return Err(os("安装输入服务", e)),
        }
    };
    let acl = Descriptor::from_sddl("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x00020085;;;AU)")?;
    // SAFETY: 普通用户仅有查询权限，不能启动、停止或重配置 SYSTEM 服务。
    unsafe { SetServiceObjectSecurity(service.0, DACL_SECURITY_INFORMATION, acl.raw()) }
        .map_err(|e| os("保护输入服务", e))?;
    // SAFETY: 无服务启动参数；服务不能被客户端用参数重新定向。
    if let Err(e) = unsafe { StartServiceW(service.0, None) }
        && e.code() != ERROR_SERVICE_ALREADY_RUNNING.to_hresult()
    {
        return Err(os("启动输入服务", e));
    }
    wait_state(&service, SERVICE_RUNNING)
}

pub fn prepare(directory: &std::path::Path) -> Result<()> {
    // 先确认调用者有安装服务的权限，才允许创建应用安装目录。
    let _manager = manager(SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE)?;
    security::prepare_directory(directory)
}

pub fn stop() -> Result<()> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    // SAFETY: 仅打开固定服务；未安装视为已停止。
    let service = match unsafe {
        OpenServiceW(
            manager.0,
            &HSTRING::from(NAME),
            SERVICE_STOP | SERVICE_QUERY_STATUS,
        )
    } {
        Ok(handle) => ServiceHandle(handle),
        Err(e) if e.code() == ERROR_SERVICE_DOES_NOT_EXIST.to_hresult() => return Ok(()),
        Err(e) => return Err(os("停止输入服务", e)),
    };
    let mut status = SERVICE_STATUS::default();
    // SAFETY: 控制固定服务，不强行杀普通用户后台。
    if let Err(e) = unsafe { ControlService(service.0, SERVICE_CONTROL_STOP, &mut status) }
        && e.code() != ERROR_SERVICE_NOT_ACTIVE.to_hresult()
    {
        return Err(os("请求输入服务停止", e));
    }
    wait_state(&service, SERVICE_STOPPED)
}

pub fn remove() -> Result<()> {
    stop()?;
    let manager = manager(SC_MANAGER_CONNECT)?;
    // SAFETY: 固定服务，删除登记后清除机器级输入授权，不删除用户配置或配对。
    unsafe {
        match OpenServiceW(
            manager.0,
            &HSTRING::from(NAME),
            windows::Win32::Storage::FileSystem::DELETE.0,
        ) {
            Ok(handle) => {
                let service = ServiceHandle(handle);
                DeleteService(service.0).map_err(|e| os("卸载输入服务", e))?;
            }
            Err(e) if e.code() == ERROR_SERVICE_DOES_NOT_EXIST.to_hresult() => {}
            Err(e) => return Err(os("打开待卸载输入服务", e)),
        }
    }
    super::policy::clear()
}

fn wait_state(service: &ServiceHandle, wanted: SERVICE_STATUS_CURRENT_STATE) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut status = SERVICE_STATUS::default();
        // SAFETY: 借用有效服务句柄进行状态查询。
        unsafe { QueryServiceStatus(service.0, &mut status) }
            .map_err(|e| os("查询输入服务状态", e))?;
        if status.dwCurrentState == wanted {
            return Ok(());
        }
        if Instant::now() >= deadline
            || (wanted == SERVICE_RUNNING && status.dwCurrentState == SERVICE_STOPPED)
        {
            return Err(failure("Windows 输入服务没有进入预期状态"));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn run() -> Result<()> {
    let name = HSTRING::from(NAME);
    let entries = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: PWSTR(name.as_ptr().cast_mut()),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW::default(),
    ];
    // SAFETY: 表以空项结尾；名称和函数在调用返回前一直有效。
    unsafe { StartServiceCtrlDispatcherW(entries.as_ptr()) }.map_err(|e| os("连接 SCM", e))
}

unsafe extern "system" fn control(
    code: u32,
    _: u32,
    _: *mut core::ffi::c_void,
    _: *mut core::ffi::c_void,
) -> u32 {
    if matches!(code, SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN) {
        STOP.store(true, Ordering::Release);
    }
    0
}

unsafe extern "system" fn service_main(_: u32, _: *mut PWSTR) {
    STOP.store(false, Ordering::Release);
    // SAFETY: 仅向 SCM 登记固定处理器；它只写原子标志，不执行阻塞清理。
    let Ok(handle) =
        (unsafe { RegisterServiceCtrlHandlerExW(&HSTRING::from(NAME), Some(control), None) })
    else {
        return;
    };
    status(handle, SERVICE_START_PENDING, 0);
    let result = service_loop(handle);
    status(handle, SERVICE_STOPPED, u32::from(result.is_err()));
}

fn status(handle: SERVICE_STATUS_HANDLE, state: SERVICE_STATUS_CURRENT_STATE, error: u32) {
    let status = SERVICE_STATUS {
        dwServiceType: SERVICE_WIN32_OWN_PROCESS,
        dwCurrentState: state,
        dwControlsAccepted: if state == SERVICE_RUNNING {
            SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN
        } else {
            0
        },
        dwWin32ExitCode: if error == 0 { 0 } else { 1066 },
        dwServiceSpecificExitCode: error,
        dwCheckPoint: if matches!(state, SERVICE_START_PENDING | SERVICE_STOP_PENDING) {
            1
        } else {
            0
        },
        dwWaitHint: if state == SERVICE_STOP_PENDING {
            5000
        } else {
            1000
        },
    };
    // SAFETY: 有效 SCM 状态句柄和完整结构。
    let _ = unsafe { SetServiceStatus(handle, &status) };
}

pub(super) fn system_environment() -> Result<()> {
    security::require_system()?;
    // SAFETY: 高权限进程不从工作目录、用户 PATH 或用户可写目录搜索 DLL。
    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32)
    }
    .map_err(|e| os("限定输入服务 DLL 搜索", e))
}

fn service_loop(handle: SERVICE_STATUS_HANDLE) -> Result<()> {
    system_environment()?;
    let installed = Installed::load()?;
    status(handle, SERVICE_RUNNING, 0);
    let mut agent: Option<(u32, (String, u64), AgentProcess)> = None;
    let mut retry = Instant::now();
    while !STOP.load(Ordering::Acquire) {
        let session = security::active_session();
        let user = security::session_identity(session).ok();
        if agent.as_ref().is_some_and(|(id, sid, process)| {
            *id != session || Some(sid) != user.as_ref() || process.exited()
        }) {
            agent = None;
        }
        if agent.is_none()
            && let Some(user) = user
            && Instant::now() >= retry
        {
            if let Ok(process) = AgentProcess::launch(&installed, session, &user.0) {
                agent = Some((session, user, process));
            }
            retry = Instant::now() + Duration::from_secs(2);
        }
        thread::sleep(Duration::from_millis(100));
    }
    status(handle, SERVICE_STOP_PENDING, 0);
    drop(agent);
    Ok(())
}
