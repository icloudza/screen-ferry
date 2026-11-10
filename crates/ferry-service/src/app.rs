//! 按平台选择入口；所有权限和 Win32 操作留在平台适配器。

use crate::cli::Mode;

pub fn init_output() {
    #[cfg(target_os = "windows")]
    ferry_platform_windows::attach_console(false);
}

pub fn run(mode: Mode) -> Result<(), String> {
    if mode == Mode::Help {
        println!(
            "screenferry-service install | stop | remove\nscreenferry-service authorize <用户SID> <控制端公开授权码>\nscreenferry-service revoke <用户SID> <设备完整指纹>\nWindows 安全输入服务（管理操作需要管理员权限）。授权码必须从控制端经可信渠道核对，普通配对不授予 UAC 权限。"
        );
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        use ferry_platform_windows as platform;
        match mode {
            Mode::Run => platform::run_input_service(),
            Mode::Install => platform::install_input_service(),
            Mode::Prepare { directory } => platform::prepare_input_service(&directory),
            Mode::Stop => platform::stop_input_service(),
            Mode::Remove => platform::remove_input_service(),
            Mode::Authorize { user_sid, code } => platform::authorize_input_peer(&user_sid, &code),
            Mode::Revoke {
                user_sid,
                fingerprint,
            } => platform::revoke_input_peer(&user_sid, &fingerprint),
            Mode::Agent {
                parent_pid,
                stop_event,
            } => platform::run_input_agent(parent_pid, stop_event),
            Mode::Help => unreachable!("已处理帮助"),
        }
        .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err("安全桌面输入服务只支持 Windows".into())
}
