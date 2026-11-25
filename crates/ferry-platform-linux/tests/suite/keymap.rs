use ferry_platform_linux::{
    evdev_to_hid, hid_to_evdev, keymap_text, lock_keys_from_xkb, wayland_key_to_hid,
};
use ferry_types::{KeyCode, LockKeys};

#[test]
fn common_keys_roundtrip() {
    let cases = [
        (30, KeyCode::keyboard(0x04)),
        (16, KeyCode::keyboard(0x14)),
        (17, KeyCode::keyboard(0x1A)),
        (18, KeyCode::keyboard(0x08)),
        (19, KeyCode::keyboard(0x15)),
        (20, KeyCode::keyboard(0x17)),
        (21, KeyCode::keyboard(0x1C)),
        (22, KeyCode::keyboard(0x18)),
        (23, KeyCode::keyboard(0x0C)),
        (24, KeyCode::keyboard(0x12)),
        (25, KeyCode::keyboard(0x13)),
        (31, KeyCode::keyboard(0x16)),
        (32, KeyCode::keyboard(0x07)),
        (33, KeyCode::keyboard(0x09)),
        (34, KeyCode::keyboard(0x0A)),
        (35, KeyCode::keyboard(0x0B)),
        (36, KeyCode::keyboard(0x0D)),
        (37, KeyCode::keyboard(0x0E)),
        (38, KeyCode::keyboard(0x0F)),
        (48, KeyCode::keyboard(0x05)),
        (46, KeyCode::keyboard(0x06)),
        (32, KeyCode::keyboard(0x07)),
        (18, KeyCode::keyboard(0x08)),
        (33, KeyCode::keyboard(0x09)),
        (34, KeyCode::keyboard(0x0A)),
        (35, KeyCode::keyboard(0x0B)),
        (23, KeyCode::keyboard(0x0C)),
        (36, KeyCode::keyboard(0x0D)),
        (37, KeyCode::keyboard(0x0E)),
        (38, KeyCode::keyboard(0x0F)),
        (50, KeyCode::keyboard(0x10)),
        (49, KeyCode::keyboard(0x11)),
        (24, KeyCode::keyboard(0x12)),
        (25, KeyCode::keyboard(0x13)),
        (16, KeyCode::keyboard(0x14)),
        (19, KeyCode::keyboard(0x15)),
        (31, KeyCode::keyboard(0x16)),
        (20, KeyCode::keyboard(0x17)),
        (22, KeyCode::keyboard(0x18)),
        (47, KeyCode::keyboard(0x19)),
        (17, KeyCode::keyboard(0x1A)),
        (45, KeyCode::keyboard(0x1B)),
        (21, KeyCode::keyboard(0x1C)),
        (44, KeyCode::keyboard(0x1D)),
        (26, KeyCode::keyboard(0x2F)),
        (27, KeyCode::keyboard(0x30)),
        (43, KeyCode::keyboard(0x31)),
        (44, KeyCode::keyboard(0x1D)),
        (45, KeyCode::keyboard(0x1B)),
        (46, KeyCode::keyboard(0x06)),
        (47, KeyCode::keyboard(0x19)),
        (48, KeyCode::keyboard(0x05)),
        (49, KeyCode::keyboard(0x11)),
        (50, KeyCode::keyboard(0x10)),
        (51, KeyCode::keyboard(0x36)),
        (52, KeyCode::keyboard(0x37)),
        (53, KeyCode::keyboard(0x38)),
        (55, KeyCode::keyboard(0x55)),
        (86, KeyCode::keyboard(0x64)),
        (121, KeyCode::keyboard(0x90)),
        (2, KeyCode::keyboard(0x1E)),
        (57, KeyCode::keyboard(0x2C)),
        (28, KeyCode::keyboard(0x28)),
        (29, KeyCode::LEFT_CTRL),
        (42, KeyCode::LEFT_SHIFT),
        (56, KeyCode::LEFT_ALT),
        (125, KeyCode::LEFT_GUI),
        (97, KeyCode::RIGHT_CTRL),
        (54, KeyCode::RIGHT_SHIFT),
        (100, KeyCode::RIGHT_ALT),
        (126, KeyCode::RIGHT_GUI),
        (59, KeyCode::keyboard(0x3A)),
        (68, KeyCode::keyboard(0x43)),
        (87, KeyCode::keyboard(0x44)),
        (88, KeyCode::keyboard(0x45)),
        (183, KeyCode::keyboard(0x68)),
        (194, KeyCode::keyboard(0x73)),
        (102, KeyCode::keyboard(0x4A)),
        (103, KeyCode::keyboard(0x52)),
        (104, KeyCode::keyboard(0x4B)),
        (105, KeyCode::keyboard(0x50)),
        (106, KeyCode::keyboard(0x4F)),
        (107, KeyCode::keyboard(0x4D)),
        (108, KeyCode::keyboard(0x51)),
        (109, KeyCode::keyboard(0x4E)),
        (110, KeyCode::keyboard(0x49)),
        (111, KeyCode::keyboard(0x4C)),
        (113, KeyCode::MUTE),
        (114, KeyCode::VOLUME_DOWN),
        (115, KeyCode::VOLUME_UP),
        (163, KeyCode::NEXT_TRACK),
        (164, KeyCode::PLAY_PAUSE),
        (165, KeyCode::PREVIOUS_TRACK),
        (166, KeyCode::STOP),
    ];
    for (evdev, hid) in cases {
        assert_eq!(evdev_to_hid(evdev), Some(hid), "evdev {evdev}");
        assert_eq!(hid_to_evdev(hid), Some(evdev), "HID {}", hid.raw());
    }
    assert_eq!(evdev_to_hid(112), None, "KEY_BRIGHTNESSDOWN 不是静音");
    assert_eq!(evdev_to_hid(0), None);
    assert_eq!(evdev_to_hid(u32::MAX), None);
    assert_eq!(hid_to_evdev(KeyCode::new(1, 0)), None);
}

#[test]
fn wayland_keycodes_are_evdev_without_an_xkb_offset() {
    let cases = [
        (30, KeyCode::keyboard(0x04)),
        (6, KeyCode::keyboard(0x22)),
        (14, KeyCode::keyboard(0x2A)),
        (38, KeyCode::keyboard(0x0F)),
        (42, KeyCode::LEFT_SHIFT),
    ];
    for (key, hid) in cases {
        assert_eq!(wayland_key_to_hid(key), Some(hid), "Wayland key {key}");
    }
    assert_eq!(wayland_key_to_hid(0), None);
    assert_eq!(wayland_key_to_hid(255), None);
    assert_eq!(wayland_key_to_hid(u32::MAX), None);
}

#[test]
fn xkb_lock_bits_ignore_transient_modifiers() {
    assert_eq!(lock_keys_from_xkb(1 | 4 | 8 | 64), LockKeys::empty());
    assert_eq!(
        lock_keys_from_xkb(2 | 16 | 128),
        LockKeys::CAPS | LockKeys::NUM | LockKeys::SCROLL,
    );
}

#[test]
fn virtual_keyboard_keymap_uses_evdev_codes() {
    for offset in 0..12 {
        let evdev = 183 + offset;
        let hid = KeyCode::keyboard(0x68 + offset as u16);
        assert_eq!(evdev_to_hid(evdev), Some(hid));
        assert_eq!(hid_to_evdev(hid), Some(evdev));
    }
    let keymap = keymap_text();
    assert_eq!(keymap.as_bytes().last(), Some(&0));
    assert!(keymap.contains("include \"evdev\""));
    assert!(keymap.contains("include \"pc+us+inet(evdev)\""));
    assert!(!keymap.contains("<I163>"));
}
