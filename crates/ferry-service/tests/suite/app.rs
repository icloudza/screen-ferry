#[cfg(all(target_os = "windows", not(feature = "experimental-secure-input")))]
#[test]
fn standard_build_cannot_install_or_run_the_privileged_channel() {
    use ferry_service::{Mode, run};

    // 这些入口必须在调用 SCM、打开桌面或创建代理之前拒绝，不需要管理员，也不改变主机。
    for mode in [
        Mode::Install,
        Mode::Run,
        Mode::Agent {
            parent_pid: 1,
            stop_event: 1,
        },
        Mode::Authorize {
            user_sid: "S-1-5-18".into(),
            code: "not-an-authorization".into(),
        },
    ] {
        let error = run(mode).unwrap_err();
        assert!(error.contains("实验阶段"), "{error}");
    }
}
