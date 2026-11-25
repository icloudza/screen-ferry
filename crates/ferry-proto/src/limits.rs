//! 协议上限。编码和解码都会检查，超出即视为错误，保证内存占用有界。

/// 设备名的最大字节数（UTF-8）。
pub const MAX_NAME_LEN: usize = 128;

/// 程序版本号（`AppInfo`）的最大字节数。
pub const MAX_VERSION_LEN: usize = 64;

/// 对端系统标识（`PlatformInfo`）的最大字节数。
pub const MAX_PLATFORM_LEN: usize = 16;

/// 一台设备最多声明的显示器数量。
pub const MAX_DISPLAYS: usize = 32;

/// 一次剪贴板通告中最多的格式数量。
pub const MAX_CLIPBOARD_FORMATS: usize = 16;

/// 单个数据分块的最大字节数。
pub const MAX_CHUNK_LEN: usize = 64 * 1024;

/// 文件清单中单个路径的最大字节数（UTF-8），也是文件通告中名字的上限。
pub const MAX_PATH_LEN: usize = 1024;

/// 一条清单消息中最多的条目数。每条最多约 1 KiB，整条不会超过数据连接的单帧上限。
pub const MAX_FILE_LIST: usize = 32;

/// 配对随机数与承诺的字节数。
pub const PAIRING_NONCE_LEN: usize = 32;

/// 输入连接（以及配对连接）上单帧消息体的最大字节数。
pub const MAX_INPUT_PAYLOAD: usize = 4 * 1024;

/// 数据连接和文件连接上单帧消息体的最大字节数：一个满载分块再加少量字段。
pub const MAX_DATA_PAYLOAD: usize = MAX_CHUNK_LEN + 64;
