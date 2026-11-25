use std::os::fd::AsRawFd;

use ferry_platform_linux::shared_file;

#[test]
fn compositor_can_map_shared_files() {
    let payload = b"xkb_keymap { };\0";
    let (file, path) = shared_file(payload, "test").expect("create shared file");
    let length = file.metadata().expect("metadata").len() as usize;
    assert_eq!(length, payload.len());

    // 与合成器一致的两种映射：wl_shm 需要读写共享映射，keymap 需要只读私有映射。
    for (prot, flags) in [
        (libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED),
        (libc::PROT_READ, libc::MAP_PRIVATE),
    ] {
        let mapped = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                prot,
                flags,
                file.as_raw_fd(),
                0,
            )
        };
        assert_ne!(
            mapped,
            libc::MAP_FAILED,
            "mmap(prot={prot}, flags={flags}) failed: {}",
            std::io::Error::last_os_error()
        );
        let contents = unsafe { std::slice::from_raw_parts(mapped.cast::<u8>(), length) };
        assert_eq!(contents, payload);
        unsafe { libc::munmap(mapped, length) };
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn keymap_payload_is_nul_terminated() {
    let bytes = ferry_platform_linux::keymap_text().as_bytes();
    assert_eq!(bytes.last(), Some(&0));
    let (_file, path) = shared_file(bytes, "xkb").expect("create keymap file");
    assert_eq!(std::fs::read(&path).expect("read back"), bytes);
    let _ = std::fs::remove_file(path);
}

#[test]
fn write_only_fd_reproduces_the_niri_mmap_failure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.path().join("write-only"))
        .expect("write-only file");
    file.set_len(4).expect("set length");

    let mapped = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        )
    };
    assert_eq!(mapped, libc::MAP_FAILED);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EACCES)
    );
}
