//! Linux evdev 键码与 USB HID 用法的转换。

use std::fs::File;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;
use std::sync::{Mutex, OnceLock};

use ferry_types::{KeyCode, LockKeys};

const KEYS: &[(u32, KeyCode)] = &[
    (1, KeyCode::keyboard(0x29)),
    (2, KeyCode::keyboard(0x1E)),
    (3, KeyCode::keyboard(0x1F)),
    (4, KeyCode::keyboard(0x20)),
    (5, KeyCode::keyboard(0x21)),
    (6, KeyCode::keyboard(0x22)),
    (7, KeyCode::keyboard(0x23)),
    (8, KeyCode::keyboard(0x24)),
    (9, KeyCode::keyboard(0x25)),
    (10, KeyCode::keyboard(0x26)),
    (11, KeyCode::keyboard(0x27)),
    (12, KeyCode::keyboard(0x2D)),
    (13, KeyCode::keyboard(0x2E)),
    (14, KeyCode::keyboard(0x2A)),
    (15, KeyCode::keyboard(0x2B)),
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
    (26, KeyCode::keyboard(0x2F)),
    (27, KeyCode::keyboard(0x30)),
    (28, KeyCode::keyboard(0x28)),
    (29, KeyCode::LEFT_CTRL),
    (30, KeyCode::keyboard(0x04)),
    (31, KeyCode::keyboard(0x16)),
    (32, KeyCode::keyboard(0x07)),
    (33, KeyCode::keyboard(0x09)),
    (34, KeyCode::keyboard(0x0A)),
    (35, KeyCode::keyboard(0x0B)),
    (36, KeyCode::keyboard(0x0D)),
    (37, KeyCode::keyboard(0x0E)),
    (38, KeyCode::keyboard(0x0F)),
    (39, KeyCode::keyboard(0x33)),
    (40, KeyCode::keyboard(0x34)),
    (41, KeyCode::keyboard(0x35)),
    (42, KeyCode::LEFT_SHIFT),
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
    (54, KeyCode::RIGHT_SHIFT),
    (55, KeyCode::keyboard(0x55)),
    (56, KeyCode::LEFT_ALT),
    (57, KeyCode::keyboard(0x2C)),
    (58, KeyCode::CAPS_LOCK),
    (59, KeyCode::keyboard(0x3A)),
    (60, KeyCode::keyboard(0x3B)),
    (61, KeyCode::keyboard(0x3C)),
    (62, KeyCode::keyboard(0x3D)),
    (63, KeyCode::keyboard(0x3E)),
    (64, KeyCode::keyboard(0x3F)),
    (65, KeyCode::keyboard(0x40)),
    (66, KeyCode::keyboard(0x41)),
    (67, KeyCode::keyboard(0x42)),
    (68, KeyCode::keyboard(0x43)),
    (69, KeyCode::NUM_LOCK),
    (70, KeyCode::SCROLL_LOCK),
    (71, KeyCode::keyboard(0x5F)),
    (72, KeyCode::keyboard(0x60)),
    (73, KeyCode::keyboard(0x61)),
    (74, KeyCode::keyboard(0x56)),
    (75, KeyCode::keyboard(0x5C)),
    (76, KeyCode::keyboard(0x5D)),
    (77, KeyCode::keyboard(0x5E)),
    (78, KeyCode::keyboard(0x57)),
    (79, KeyCode::keyboard(0x59)),
    (80, KeyCode::keyboard(0x5A)),
    (81, KeyCode::keyboard(0x5B)),
    (82, KeyCode::keyboard(0x62)),
    (83, KeyCode::keyboard(0x63)),
    (86, KeyCode::keyboard(0x64)),
    (87, KeyCode::keyboard(0x44)),
    (88, KeyCode::keyboard(0x45)),
    (96, KeyCode::keyboard(0x58)),
    (97, KeyCode::RIGHT_CTRL),
    (98, KeyCode::keyboard(0x54)),
    (99, KeyCode::keyboard(0x46)),
    (100, KeyCode::RIGHT_ALT),
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
    (117, KeyCode::keyboard(0x67)),
    (121, KeyCode::keyboard(0x90)),
    (125, KeyCode::LEFT_GUI),
    (126, KeyCode::RIGHT_GUI),
    (127, KeyCode::keyboard(0x65)),
    (163, KeyCode::NEXT_TRACK),
    (164, KeyCode::PLAY_PAUSE),
    (165, KeyCode::PREVIOUS_TRACK),
    (166, KeyCode::STOP),
    (183, KeyCode::keyboard(0x68)),
    (184, KeyCode::keyboard(0x69)),
    (185, KeyCode::keyboard(0x6A)),
    (186, KeyCode::keyboard(0x6B)),
    (187, KeyCode::keyboard(0x6C)),
    (188, KeyCode::keyboard(0x6D)),
    (189, KeyCode::keyboard(0x6E)),
    (190, KeyCode::keyboard(0x6F)),
    (191, KeyCode::keyboard(0x70)),
    (192, KeyCode::keyboard(0x71)),
    (193, KeyCode::keyboard(0x72)),
    (194, KeyCode::keyboard(0x73)),
    (224, KeyCode::consumer(0x70)),
    (225, KeyCode::consumer(0x6F)),
];

/// 将 Linux evdev 键码转换为协议使用的 HID 键码。
pub fn evdev_to_hid(evdev: u32) -> Option<KeyCode> {
    KEYS.iter()
        .find_map(|&(code, hid)| (code == evdev).then_some(hid))
}

/// Wayland `wl_keyboard.key` 已经是 evdev 键码，不是需要减 8 的 XKB 键码。
pub fn wayland_key_to_hid(key: u32) -> Option<KeyCode> {
    evdev_to_hid(key)
}

/// 将协议使用的 HID 键码转换为 virtual-keyboard 所需的 evdev 键码。
pub fn hid_to_evdev(hid: KeyCode) -> Option<u32> {
    KEYS.iter()
        .find_map(|&(code, mapped)| (mapped == hid).then_some(code))
}

/// 将 XKB 的锁定修饰键位转换为协议里的锁定键。
pub fn lock_keys_from_xkb(mods_locked: u32) -> LockKeys {
    let mut locks = LockKeys::empty();
    if mods_locked & 2 != 0 {
        locks |= LockKeys::CAPS;
    }
    if mods_locked & 16 != 0 {
        locks |= LockKeys::NUM;
    }
    if mods_locked & 128 != 0 {
        locks |= LockKeys::SCROLL;
    }
    locks
}

/// 供 virtual-keyboard 使用的 XKB keymap；键码采用 evdev 编号加 8。
pub fn keymap_text() -> &'static str {
    concat!(
        "xkb_keymap {\n",
        "  xkb_keycodes { include \"evdev\" };\n",
        "  xkb_types { include \"complete\" };\n",
        "  xkb_compatibility { include \"complete\" };\n",
        "  xkb_symbols {\n",
        "    include \"pc+us+inet(evdev)\"\n",
        "  };\n",
        "};\n\0",
    )
}

#[derive(Default)]
struct ActiveKeymap {
    generation: u64,
    bytes: Vec<u8>,
}

fn active_keymap() -> &'static Mutex<ActiveKeymap> {
    static ACTIVE: OnceLock<Mutex<ActiveKeymap>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(ActiveKeymap::default()))
}

pub(crate) fn remember_keymap(bytes: Vec<u8>) {
    let mut active = active_keymap().lock().unwrap_or_else(|e| e.into_inner());
    active.generation = active.generation.wrapping_add(1);
    active.bytes = bytes;
}

pub(crate) fn current_keymap() -> (u64, Vec<u8>) {
    let active = active_keymap().lock().unwrap_or_else(|e| e.into_inner());
    (active.generation, active.bytes.clone())
}

pub(crate) fn updated_keymap(generation: u64) -> Option<(u64, Vec<u8>)> {
    let active = active_keymap().lock().unwrap_or_else(|e| e.into_inner());
    if active.generation == generation || active.bytes.is_empty() {
        return None;
    }
    Some((active.generation, active.bytes.clone()))
}

pub(crate) fn read_keymap(fd: OwnedFd, size: u32) -> io::Result<Vec<u8>> {
    let size = size as usize;
    if size == 0 || size > 16 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "XKB keymap 大小无效",
        ));
    }
    let mut bytes = vec![0; size];
    // 合成器可能把 fd 的读写位置留在末尾；协议要求从偏移 0 读取 keymap。
    File::from(fd).read_exact_at(&mut bytes, 0)?;
    if bytes.last() != Some(&0) {
        bytes.push(0);
    }
    Ok(bytes)
}
