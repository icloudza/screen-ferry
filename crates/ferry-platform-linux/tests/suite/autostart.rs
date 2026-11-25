use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use ferry_platform::Autostart;
use ferry_platform_linux::LinuxAutostart;

#[test]
fn desktop_exec_is_escaped_and_written_privately() {
    let directory = tempfile::tempdir().unwrap();
    let _environment = Environment::set("XDG_CONFIG_HOME", directory.path());
    let autostart = LinuxAutostart::new(vec![
        "/opt/Screen Ferry/bin/screenferry".into(),
        "--flag=a b".into(),
        "\"quoted\"".into(),
        "$HOME".into(),
        "`id`".into(),
        "back\\slash".into(),
        "100%".into(),
        "$(id);semi".into(),
        String::new(),
    ])
    .unwrap();

    autostart.set_enabled(true).unwrap();
    let path = directory.path().join("autostart/screenferry.desktop");
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains(
        r#"Exec="/opt/Screen Ferry/bin/screenferry" "--flag=a b" "\"quoted\"" "\$HOME" "\`id\`" "back\\slash" "100%%" "\$(id);semi" """#
    ));
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(autostart.is_enabled().unwrap());

    autostart.set_enabled(false).unwrap();
    assert!(!path.exists());
    assert!(!autostart.is_enabled().unwrap());
}

struct Environment {
    key: &'static str,
    previous: Option<OsString>,
}

impl Environment {
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        let previous = std::env::var_os(key);
        // SAFETY: 此测试是唯一修改该变量的测试，且恢复 guard 先于临时目录释放。
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        // SAFETY: 与 set 相同，此时没有其他线程读取该变量。
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
