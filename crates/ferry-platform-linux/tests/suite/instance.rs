use ferry_platform_linux::LinuxInstanceLock;

#[test]
fn only_one_process_can_hold_the_lock() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("instance.lock");
    let first = LinuxInstanceLock::acquire(&path).unwrap();
    assert!(first.is_some());
    assert!(LinuxInstanceLock::acquire(&path).unwrap().is_none());
    drop(first);
    assert!(LinuxInstanceLock::acquire(&path).unwrap().is_some());
}
