# Arch Linux / niri 与 macOS 互通交接

日期：2026-11-25  
适用代码：本次工作区，workspace 版本 0.2.0，线协议 `WIRE_FORMAT = 1`。

## 1. 本次更新

- 新增 `ferry-platform-linux`，面向 Arch Linux 的 niri / Wayland 会话。
- 通过 layer-shell、exclusive keyboard、pointer constraints、relative pointer、virtual pointer 和 virtual keyboard 共享键鼠，不需要 root、uinput 或 evdev 权限。
- 支持剪贴板同步、普通文件传输和命令行发送文件；Linux 不声明边缘拖放能力。
- 新增 Flutter Linux 宿主、KSNI 托盘、用户级 XDG 自启、单实例保护和 Linux 打包。
- 新增能力协商的 `PlatformInfo`，连接后显示对端真实系统。旧版 macOS / Windows 对端不会收到该消息，连接与原有功能不受影响；对端系统会显示为未知。
- Linux 使用 XKB / Ctrl 习惯，与 macOS 的 Command / Ctrl 转换以双方实际系统为依据。
- 虚拟键盘使用 compositor 当前 XKB keymap，覆盖非美式布局和布局切换；锁定键只读取 `mods_locked`。
- 修复重复抓取的旧指针约束、输出旋转或分辨率变化后的输入区域，以及连续 / 高分辨率滚轮累积。
- Linux 键盘原始事件直接按 evdev 映射为 HID，不再误减 XKB 偏移 8；进入远端同时确认指针锁定、键盘焦点与 niri 快捷键抑制，焦点或抑制意外失效时立即回本机并松开对端按键。
- 剪贴板平台调用移出异步运行时；Linux 读写串行化，断开的旧会话不会在之后覆盖新会话内容。

协议格式标记没有改变。新增系统信息使用独立能力位协商，不需要两台设备同时升级。

## 2. 当前 Linux 能否安装

**目前没有 pacman 包、AUR 包、AppImage、deb/rpm 或桌面安装程序。** 当前可用方式有两种：

1. **完整便携运行**：在 Arch 上安装依赖并执行 `cargo xtask bundle`，得到 `target/bundle/linux/ScreenFerry/`。当前容器已成功生成该目录（约 34 MB），可整个目录复制到 `~/.local/opt/ScreenFerry/` 后运行，不需要 root，但不会自动登记系统应用菜单或桌面图标。
2. **后台单文件试运行**：当前容器已生成 `target/release/screenferry`（x86_64，约 10 MB）。可复制到 Arch 的 niri 会话中直接运行，但它不包含 Flutter 设置界面，不能替代完整包；若 Arch 的 glibc 不兼容，就在 Arch 上重新编译。

运行完整版需要：

```bash
sudo pacman -S --needed base-devel clang cmake ninja pkgconf gtk3 wl-clipboard xdg-utils
```

托盘使用 KSNI，niri 桌面需要有 StatusNotifierItem 宿主，例如 Waybar 的 KSNI 模块。没有托盘宿主时，后台仍可运行，但托盘菜单不可见。

完整便携运行示例：

```bash
cd ~/.local/opt/ScreenFerry
./screenferry
```

设置界面为 `screenferry-settings`，与后台位于同一目录。后台启动时会尝试打开它。也可以在设置界面中开启登录自启，自启文件写入 `~/.config/autostart/screenferry.desktop`；移动安装目录后应关闭再重新开启自启。

Linux 暂不支持边缘拖放和应用内自动更新。文件用 `./screenferry send <文件或文件夹>` 发送，更新通过重新构建或以后的软件包完成。

niri 25.02 及以上版本支持键盘快捷键抑制。ScreenFerry 抓取时会暂停普通全局快捷键，把完整组合键转发给远端；合成器仍可能保留自己的特殊逃生组合键。建议在 `~/.config/niri/binds.kdl` 中加入不会被抑制的逃生绑定：

```kdl
Mod+Escape allow-inhibiting=false { toggle-keyboard-shortcuts-inhibit; }
```

按该组合键会恢复 niri 快捷键，ScreenFerry 随即把指针和键盘交还本机。

## 3. 把源码带到 macOS

不要复制 `target/`、`app/build/` 或 `app/.dart_tool/`。在 Linux 项目根目录执行，例如：

```bash
rsync -a \
  --exclude target \
  --exclude app/build \
  --exclude app/.dart_tool \
  ./ <macOS用户名>@<macOS地址>:~/Projects/screen-ferry-Linux/
```

macOS 需要 Xcode 命令行工具、Rust 1.91.0 和 Flutter 3.47。本地化文件已在 Linux 容器用 Flutter 3.47.9 / Dart 3.13.7 重新生成并通过检查；macOS 上仍建议按本机环境重新生成、格式化和编译：

```bash
cd ~/Projects/screen-ferry-Linux

rustup toolchain install 1.91.0 --profile minimal --component rustfmt,clippy
rustup default 1.91.0

cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask lint

cd app
dart format lib test
flutter gen-l10n
flutter analyze
flutter test
cd ..

cargo xtask bundle
```

本次已执行 `flutter gen-l10n`，生成文件以工具结果为准；macOS 上如果 SDK 小版本不同，重新生成后仍需检查差异。

本机架构的构建产物：

```text
target/bundle/macos/ScreenFerry.app
```

直接打开：

```bash
open target/bundle/macos/ScreenFerry.app
```

如需通用包，再执行：

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo xtask bundle --universal
```

本次只需验证互通，建议先构建本机架构，不必公证。没有 Developer ID 证书时会使用临时签名；每次重新构建后，都要在「系统设置 → 隐私与安全性 → 辅助功能」中重新允许 ScreenFerry。局域网发现还需要允许 macOS 的本地网络访问。

## 4. 双机测试顺序

推荐先验证 **Mac 控制 Linux**，再验证 **Linux 控制 Mac**。两台机器使用同一份源码构建，并连接到同一局域网。

### 4.1 启动与配对

1. Mac 打开构建出的 `ScreenFerry.app`，Linux 在 niri 终端运行 `./screenferry`。
2. 两端确认托盘或设置界面能打开。Linux 托盘不可见时先检查 Waybar 的 KSNI 模块。
3. 在「设备」页确认两端分别显示 `macOS` 和 `Linux`。若对端是旧版程序，显示未知属于预期行为。
4. 核对同一个 6 位配对码，记录是否首次连接成功、重连是否无需重新配对。
5. 若 mDNS 找不到，使用「按地址连接」或邀请功能，记录失败端和地址。

### 4.2 Mac → Linux

- 将 Mac 设为控制端，Linux 设为被控制端，把 Linux 屏幕摆在 Mac 右侧。
- 从 Mac 右边缘推出光标，确认进入 Linux，反向推回后能回到原位置。
- 验证鼠标移动、左右键、中键、拖拽、窗口调整和滚轮；检查高分辨率触控板滚动是否丢失。
- 在 Linux 文本编辑器输入英文字母、数字、符号、Enter、Backspace、Tab、Esc。
- 验证 Command+C / V 到 Linux 后是否按 XKB / Ctrl 习惯工作；分别测试左右 Shift、Ctrl、Alt、Command。
- 验证方向键、Home / End、Ctrl+← / →、媒体键和 Caps / Num / Scroll Lock。
- 反复来回切换至少 10 次，确认不会卡键、重复抓取不会断开 Wayland。
- 若有旋转显示器或切换分辨率，重复进入 / 退出并检查边缘捕获区域。
- 使用非 US 布局切换输入法或键盘布局，再发送物理位置相同的键；若切换布局后第一次输入异常，重点记录 keymap 刷新时机。

### 4.3 Linux → Mac

- 交换控制角色后重复光标切换、鼠标、拖拽、滚轮和键盘测试。
- 在 Linux 文本编辑器依次输入 `a`、`5`、`l`、Backspace 和 Shift+字母，确认 macOS 收到的字符与物理按键一致；重点回归此前 `a` 输出 `5` 的故障。
- 进入 macOS 后确认 Arch 前台窗口不再收到键盘；切换窗口或让 overlay 失去焦点时，光标应立即回到 Arch，且 macOS 松开所有按键。
- 在 macOS 上测试 `Cmd+Q`、`Cmd+Space`、`Cmd+Tab` 以及 niri 中已绑定的 `Mod+Q` / `Mod+Space`，确认 Arch 不执行关闭窗口、切换布局或启动器，macOS 收到完整组合键。
- 抓取期间按配置的 `Mod+Escape`，确认 niri 立即恢复快捷键且光标回到 Arch；鼠标再次进入 macOS 后快捷键抑制重新生效，返回 Arch 后本机快捷键恢复正常。
- 验证 Linux 的 Ctrl 快捷键到 Mac 后是否按设置映射为 Command。
- 断开网络或退出对端后台，确认本机立即收回控制并松开按下的键；恢复后能自动重连。

### 4.4 剪贴板

- 普通中文、英文和 emoji 文本双向同步。
- 大文本与图片双向同步，确认格式、内容和大小正确。
- 光标不切换时，不应覆盖另一端剪贴板。
- 连续多次复制只同步最后一次；复制不支持的空内容应清除旧通告，不应粘贴旧内容。
- 调整剪贴板大小上限后无需重连即可生效。
- Linux 当前一次只写一个 MIME 类型，优先 PNG、HTML、RTF、TEXT；同时带文字和图片的富内容在 Linux 上不保证所有格式同时保留。

### 4.5 文件

Linux 端执行：

```bash
./screenferry send ~/Downloads/测试文件.txt
```

确认 Mac 收到文件，再反向从 Mac 使用现有文件功能发送。Linux 边缘拖放暂不支持，不要把拖放失败当作本次回归。

### 4.6 常驻与自启

- 重复启动两次 `./screenferry`，第二次应把现有设置界面带到前台后退出，不能出现两个后台争抢输入。
- 开启自启后检查 `~/.config/autostart/screenferry.desktop`，注销并重新登录验证。
- Linux 使用 `./screenferry quit` 退出；macOS 使用托盘菜单退出。

### 4.7 协议自检失败

后台启动提示「当前 Wayland 合成器缺少 ScreenFerry 所需协议」时，反馈缺少的完整接口名。必需的接口是：
`zwlr_layer_shell_v1`、`zwp_pointer_constraints_v1`、`zwp_relative_pointer_manager_v1`、`zwlr_virtual_pointer_manager_v1`、`zwp_virtual_keyboard_manager_v1`。
虚拟指针来自 wlr-protocols（`zwlr_` 前缀），虚拟键盘来自 weston 协议（`zwp_` 前缀），前缀不同是协议本身的命名，不能按同名前缀推断。
用 `wayland-info`（部分发行版由 `wayland-utils` 包提供）列出当前会话公布的接口，再与上面的清单对照。

若错误是 `Backend error: Protocol error ... wl_shm ... Failed to mmap fd`，表示合成器无法映射客户端传来的共享内存 fd。
先确认后台已换成最新构建；需要定位时从终端运行 `WAYLAND_DEBUG=1 ~/.local/share/screenferry/ScreenFerry/screenferry -v`
并保留完整协议日志，尤其是断开前最后一个请求与出错对象。wl_shm 缓冲区和 XKB keymap 的 fd 都必须可读，只写打开会让 mmap 以 `EACCES` 失败。

## 5. 日志与数据位置

| 平台 | 配置与配对 | 日志与运行时文件 |
|---|---|---|
| macOS | `~/Library/Application Support/ScreenFerry/` | `~/Library/Caches/ScreenFerry/` |
| Linux | `${XDG_DATA_HOME:-~/.local/share}/screenferry/` | `${XDG_CACHE_HOME:-~/.cache}/screenferry/` |

调试运行：

```bash
# Linux
./screenferry -v

# macOS 打包后的后台
target/bundle/macos/ScreenFerry.app/Contents/MacOS/screenferry -v
```

优先从设置界面「日志 → 运行日志」导出诊断文件。不要直接共享两台的 `identity/` 目录，设备证书必须各自生成、通过配对建立信任。排查输入问题时记录：方向、操作步骤、是否首次发生、对端系统、键盘布局、是否旋转屏幕、能否重连，以及两端日志时间。Linux 日志会记录 Wayland 原始键码、HID、键盘焦点和指针锁定状态；macOS 日志记录 HID 与注入键码，可逐级核对。

## 6. 实机验证记录（2026-11-25）

### 6.1 测试系统

| 项目 | Linux 控制端 | macOS 被控端 |
|---|---|---|
| 系统 | Arch Linux，x86_64 | macOS 15.8.3（24H241），x86_64 MacBook Pro |
| 内核 / 桌面 | Linux `7.3.4-arch1-1`；niri `v26.04-322-g2fbc6f3b-modified`（Wayland） | 原生 macOS 图形会话 |
| 处理器 | Intel Core i5-12400F | 未记录 |
| 显示器 | 单个逻辑输出，1920×1080，100% 缩放 | 握手报告单个逻辑显示器，1920×1080，100% 缩放 |
| 相对位置 | Mac 屏幕位于 Linux 左侧；Linux 以 `--side left` 运行 | 通过局域网连接 Linux |
| 构建工具 | Rust 1.91.0；Flutter 3.47.9 / Dart 3.13.7 | Rust 1.91.0；Flutter 3.47.9 / Dart 3.13.7；Xcode 26.3 |

连接使用同一局域网、mDNS 发现和 TLS 1.3。最终复测时 Linux 使用发布构建 `8b61beda657c15deb5f24d66e82ad4f2a0b41541f74a3f188f5a2add07b091c2`，Mac 使用用户级应用；旧后台通过内置 `SIGINT` 路径优雅退出后完成替换。
本文记录的是实际测试环境，不代表所有 Arch 发行版版本、niri 版本或显示器组合都已覆盖。

### 6.2 已通过

- **双向边缘切换**：Linux 左边缘推到 Mac、Mac 右边缘推回 Linux，以及多次连续往返均成功；日志和接收端指针坐标都确认控制权交接。
- **Linux 捕获链**：niri 的 layer-shell 输入区域、pointer constraints 锁定、relative pointer、虚拟指针和虚拟键盘初始化通过；锁定后事件正确标记为 `grabbed=true`。
- **位移质量**：Wayland `f64` 亚像素位移先累积再发送，慢速移动和反向移动不再逐事件丢失。
- **显示器协商**：Linux 与 Mac 均报告 1920×1080、100% 缩放；Mac 在活动显示器列表为空时回退到在线显示器列表。
- **文本剪贴板**：完成一次 Linux→Mac 文本同步；未覆盖图片、多 MIME 和频繁更新。
- **Linux 生命周期**：独立数据目录下，最新后台完成 IPC 就绪、设置界面连接和官方 `screenferry quit` 优雅退出。
- **构建与静态检查**：最终 Linux 全工作区 393 项 Rust 测试通过、2 项忽略；`cargo fmt --all --check`、全工作区 Clippy（`-D warnings`）、`cargo xtask lint` 和 macOS 目标编译检查均通过。Flutter 侧此前完成 120 项测试、分析、Linux/macOS release 构建。
- **实机修复范围**：wlr 虚拟指针协议、`wl_shm`/XKB fd 可读性、layer-shell configure 时序、`Current|Preferred` 输出模式、启动与 grab/release 确认、Mac 休眠显示器回退均已纳入本次验证。
- **最终键盘复测**：Linux→macOS 普通字母、数字、符号与组合键输入正常；`Cmd+Q`、`Cmd+Space`、`Cmd+Tab` 及 niri 已绑定的 `Mod+Q` / `Mod+Space` 不再触发 Arch 本地动作。`Super+Escape` 能立即解除抑制并回到 Arch，再次进入 macOS 后独占恢复正常。

### 6.3 尚未完成或未覆盖

- 文件传输、非美式键盘布局、锁定键、滚轮方向、多显示器热插拔、屏幕旋转和长时间重连尚未完成系统化实机验收。
- `cargo deny check` 未执行成功，因为当前构建环境未安装 `cargo-deny`。

## 7. 已知限制

- Linux 富剪贴板一次只写一个 MIME 类型；边缘拖放和应用内更新未实现。
- 首次获得捕获层键盘焦点前，虚拟键盘使用内置美式 keymap；当前布局会在捕获层收到 keymap 后共享。若首次输入异常，先等待一次正常键盘焦点再复测。
- 无法观察本机初始 Caps / Num / Scroll Lock 状态时，锁定键同步会拒绝猜测，而不是发送错误状态。
