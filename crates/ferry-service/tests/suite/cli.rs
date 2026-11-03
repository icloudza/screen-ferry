use ferry_service::{Mode, parse};

fn args(items: &[&str]) -> Result<Mode, String> {
    parse(items.iter().map(|s| (*s).to_owned()))
}

#[test]
fn only_fixed_service_modes_are_accepted() {
    assert_eq!(args(&[]), Ok(Mode::Help));
    for (text, mode) in [
        ("run", Mode::Run),
        ("install", Mode::Install),
        ("stop", Mode::Stop),
        ("remove", Mode::Remove),
    ] {
        assert_eq!(args(&[text]), Ok(mode));
    }
    assert_eq!(
        args(&["agent", "42", "12"]),
        Ok(Mode::Agent {
            parent_pid: 42,
            stop_event: 12
        })
    );
    assert!(args(&["prepare", "C:\\Program Files\\ScreenFerry"]).is_ok());
}

#[test]
fn rejects_paths_arbitrary_commands_and_invalid_control_handles() {
    for input in [
        &["run", "cmd.exe"][..],
        &["install", "--path", "C:\\user.exe"],
        &["agent"],
        &["agent", "0", "12"],
        &["agent", "42", "0"],
        &["agent", "42", "-1"],
        &["agent", "42", "12", "extra"],
        &["powershell"],
        &["--config", "other"],
        &["prepare"],
        &["prepare", "C:\\Program Files\\ScreenFerry", "cmd.exe"],
        &["authorize"],
        &["authorize", "S-1-5-18"],
        &["authorize", "S-1-5-18", "short"],
        &["revoke", "S-1-5-18", "short"],
    ] {
        assert!(args(input).is_err(), "{input:?}");
    }
}
