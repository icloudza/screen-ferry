# AGENTS.md

本文件是 ScreenFerry 仓库的工程规范，人和 AI 编码助手都必须遵守。

第 3–10 节的结构规则已冻结。确需修改时，先在 `docs/adr/` 写决策记录，并经维护者确认。

---

## 1. 项目概述

ScreenFerry 是跨 Windows 和 macOS 的键鼠共享工具，同时提供剪贴板与文件同步，定位类似 Deskflow。

目标：**低内存、稳定、易扩展**。

## 2. 已定技术决策

| 方面 | 决策 |
|---|---|
| 语言 | Rust，edition 2024 |
| 传输 | TCP + rustls（TLS 1.3）；输入连接与数据连接分离；不兼容 Deskflow 协议 |
| 发现与配对 | mDNS 发现；配对码配对；配对后固定对方证书 |
| 进程模型 | 后台常驻（含托盘）；设置界面为独立进程，按需启动，关闭即退出 |
| 界面 | Flutter（自定义设计，见 ADR 0003）；工程在 `app/`，直接经本机通信连接后台（ADR 0004） |
| 托盘 | tray-icon |
| Windows | windows-rs + Win32 API |
| macOS | objc2 系列 crate |
| 存储 | TOML 配置 + SQLite；图片和文件存磁盘 |
| 本机进程通信 | 本机回环 TCP（`127.0.0.1`）+ 令牌，每行一条 JSON；协议见 `docs/ipc.md`（ADR 0004） |
| 版本更新 | GitHub Releases 上的安装包和 minisign 签名的更新清单；macOS 交换应用，Windows 静默运行安装包（ADR 0014） |
| 按键编码 | 协议中传物理键（USB HID Usage），两端各自转换成本地键码 |
| 点击与滚轮 | 事件携带当时的光标绝对位置，接收端先定位再执行 |

---

## 3. 顶层目录

```
screen-ferry/
├─ Cargo.toml              # 虚拟 workspace：只管成员、统一依赖版本、统一 lint
├─ Cargo.lock              # 提交到仓库
├─ rust-toolchain.toml     # 固定工具链版本
├─ rustfmt.toml
├─ deny.toml               # cargo-deny：许可证、漏洞、重复依赖检查
├─ .cargo/config.toml      # 别名：cargo xtask …
├─ .editorconfig
├─ .gitattributes          # 检出时统一用 LF 换行
├─ .gitignore
├─ AGENTS.md               # 本规范
├─ CLAUDE.md               # 只有一行 @AGENTS.md，供 Claude Code 读取
├─ README.md               # 英文；中文版是 README.zh-CN.md，两份同时改（ADR 0013）
├─ README.zh-CN.md
├─ LICENSE-MIT             # 双许可证：MIT OR Apache-2.0
├─ LICENSE-APACHE
├─ CHANGELOG.md
│
├─ crates/                 # 所有 Rust crate，一层平铺，不嵌套
├─ xtask/                  # 构建自动化：打包、签名、公证、发布（不写 shell 脚本）
├─ fuzz/                   # 协议解码器的 fuzz 测试（nightly，不加入 workspace）
├─ app/                    # Flutter 设置界面（不是 Cargo workspace 成员），测试放 app/test/
├─ assets/                 # 唯一存放二进制资源的地方（例外：app/ 中平台工程要求的图标，见 ADR 0003）
│  ├─ icons/               #   应用图标、托盘图标（svg 源文件 + 导出图）
│  ├─ dmg/                 #   Mac 安装盘的窗口背景（svg 源文件 + 导出图）
│  ├─ readme/              #   README 头图，中英文 × 浅色、深色（svg 源文件 + 导出图）
│  └─ promo/               #   宣传片成片（1080p60）
├─ packaging/              # 平台打包配置，只放配置，不放代码
│  ├─ macos/               #   Info.plist、entitlements.plist
│  └─ windows/             #   app.manifest（DPI、uiAccess）、安装包配置
├─ docs/
│  ├─ architecture.md      #   架构总览 + crate 地图
│  ├─ protocol.md          #   线协议说明
│  ├─ releases/            #   每个版本的中英文更新说明（ADR 0014）
│  └─ adr/                 #   架构决策记录
├─ .github/workflows/
└─ .spec-workflow/         # 本地工具目录，已加入 .gitignore，不提交
```

不得在顶层新增其他目录。

## 4. crate 分层

```
crates/
│  ── L0 基础 ───────────────────────────────────
├─ ferry-types              # 领域类型：设备/屏幕 ID、HID 键码、输入事件、几何、配置结构
│  ── L1 纯逻辑（无 IO、无异步、无平台差异）───────────
├─ ferry-proto              # 线协议：帧格式、消息、版本与能力协商
├─ ferry-core               # 屏幕布局、边缘切换、会话状态机、按键状态跟踪
├─ ferry-trust              # 设备身份、证书指纹、配对码校验、TLS 证书验证器
├─ ferry-platform           # 平台抽象 trait（只有接口，没有实现）
├─ ferry-update             # 版本更新：更新清单、签名校验、版本判断
│  ── L2 适配层（对接操作系统、网络、磁盘、进程）───────
├─ ferry-platform-windows
├─ ferry-platform-macos
├─ ferry-platform-mock      # 测试用的假平台，可以注入按键、记录输出
├─ ferry-net                # TCP + rustls、输入/数据双连接、心跳、重连、mDNS
├─ ferry-store              # 配置文件、SQLite、数据目录管理
├─ ferry-ipc                # 本机进程通信：消息定义 + 命名管道/UDS
│  ── L3 组装 ───────────────────────────────────
├─ ferry-engine             # 把 core、platform、net、store、ipc 组装成运行时
│  ── L4 应用（二进制）────────────────────────────
├─ ferry-daemon             # → screenferry：后台常驻 + 托盘
└─ ferry-service            # → screenferry-service：Windows 安全输入服务（可选，ADR 0015）
```

按规则预留、尚未创建的 crate：

- `ferry-platform-linux`（L2）

## 5. 依赖规则

1. 只能依赖更低层，不能依赖同层或上层。
2. L2 之间互不依赖：每个适配器只依赖 L0 和 L1。
3. `ferry-engine` 只依赖 `ferry-platform` 中的 trait，不依赖任何具体平台实现。具体实现由 L4 应用按 `cfg` 选择后注入。
4. 设置界面（`app/`）只通过 `ferry-ipc` 定义的协议与后台通信，不直接读写配置；配置只有后台一个写入方。
5. `#[cfg(target_os = …)]` 只允许出现在 `ferry-platform-*` 和 L4 应用中。
6. `unsafe` 只允许出现在 `ferry-platform-*` 中，其余 crate 一律 `#![forbid(unsafe_code)]`。
7. L0 和 L1 不引入 tokio，不做文件或网络 IO。
8. 所有第三方依赖的版本统一在根 `Cargo.toml` 的 `[workspace.dependencies]` 声明，各 crate 只写 `xxx.workspace = true`。

## 6. 命名规则

| 对象 | 规则 | 示例 |
|---|---|---|
| crate 目录 | 等于 package 名，`ferry-<名词>`，kebab-case | `ferry-net` |
| 平台 crate 后缀 | 与 `cfg(target_os)` 的值完全一致 | `-windows` `-macos` `-linux` |
| 可执行文件 | `screenferry[-<角色>]` | `screenferry`、`screenferry-service` |
| 源文件 / 模块 | snake_case；不使用 `mod.rs`，采用 `foo.rs` + `foo/` | `edge_switch.rs` |
| 测试文件 | 与被测模块同名，不加任何后缀 | `tests/suite/edge_switch.rs` |
| ADR | 4 位序号 + kebab 标题 | `0001-use-tcp-transport.md` |

## 7. 单个 crate 的内部结构

```
crates/ferry-xxx/
├─ Cargo.toml        # 版本、edition、依赖全部 .workspace = true
├─ build.rs          # 只在需要时才有
├─ src/              # 只放业务代码
│  ├─ lib.rs         # 只放 //! 文档、mod 声明、pub use，不写逻辑
│  ├─ error.rs       # 本 crate 的 Error 类型
│  └─ …
├─ tests/
│  └─ suite/         # 本 crate 的全部测试，见第 9 节
└─ benches/          # 可选
```

- 每个 crate 的职责写在 `lib.rs` 开头的 `//!` 文档中，不另外放 README。
- 应用 crate 的 `main.rs` 只做三件事：解析参数、初始化日志、调用 `run()`。

## 8. 平台 crate 的文件对应

`ferry-platform` 与每个平台实现使用完全相同的文件名，与 trait 一一对应：

```
ferry-platform/src/              ferry-platform-{windows,macos,mock}/src/
├─ capture.rs     InputCapture   ├─ capture.rs
├─ inject.rs      InputInject    ├─ inject.rs
├─ keymap.rs      HID ⇄ 本地键码  ├─ keymap.rs
├─ clipboard.rs   Clipboard      ├─ clipboard.rs
├─ display.rs     Displays       ├─ display.rs
├─ permission.rs  Permissions    ├─ permission.rs
├─ autostart.rs   Autostart      ├─ autostart.rs
└─ event_loop.rs  EventLoop      └─ event_loop.rs
```

新增平台能力时，trait 与所有实现同时新增同名文件。

## 9. 测试

1. **测试代码与业务代码物理隔离。** `src/` 下只放业务代码：
   - 禁止在 `src/` 的文件中写 `#[cfg(test)] mod tests`
   - 禁止在 `src/` 中放测试文件，例如 `a.rs` 旁边的 `a_test.rs`
2. 每个 crate 的测试统一放在 `tests/suite/`，编译为一个测试目标：

   ```
   crates/ferry-core/
   ├─ src/
   │  ├─ lib.rs
   │  ├─ layout.rs
   │  ├─ edge_switch.rs
   │  └─ session/
   │     └─ heartbeat.rs
   └─ tests/
      └─ suite/
         ├─ main.rs          # 只放 mod 声明
         ├─ support.rs       # 本 crate 测试共用的辅助代码
         ├─ layout.rs        # 对应 src/layout.rs
         ├─ edge_switch.rs   # 对应 src/edge_switch.rs
         └─ session/
            └─ heartbeat.rs  # 对应 src/session/heartbeat.rs
   ```

3. 测试文件的路径与被测模块在 `src/` 中的路径保持镜像，文件名相同。
4. `tests/` 顶层不放 `.rs` 文件。Cargo 会把每个顶层文件单独编译成一个测试程序，拖慢编译。
5. 测试只通过 crate 的公开接口进行。需要测试但不该公开的逻辑，说明它应该拆成更小的独立单元，通常下沉到 L1 crate。
6. 跨 crate 的场景测试（断线、丢包、卡键、控制权归还）放在 `ferry-engine/tests/suite/`，使用 `ferry-platform-mock`。
7. 性能测试放 `benches/`，fuzz 测试放仓库根目录的 `fuzz/`。
8. 文档注释中的示例代码属于文档，允许保留，会作为 doctest 运行。
9. 以上规则针对 Rust crate。`app/`（Flutter）按 Flutter 惯例把测试放在 `app/test/`。

## 10. 运行时数据目录

```
数据目录   macOS:   ~/Library/Application Support/ScreenFerry/
          Windows: %APPDATA%\ScreenFerry\
├─ config.toml       # 第一行 schema_version = 1，以后靠它做迁移
├─ peers.toml        # 已信任的设备
├─ identity/         # device.key、device.crt（只有当前用户可读）
├─ history.db        # 剪贴板历史、传输任务
└─ blobs/            # 图片和文件按内容哈希存放：ab/cdef…

缓存目录   macOS:   ~/Library/Caches/ScreenFerry/
          Windows: %LOCALAPPDATA%\ScreenFerry\
├─ ipc.json          # 后台运行时写下的端口与令牌，退出时删除（ADR 0004）
├─ logs/
├─ partial/          # 传输中的分块，可以随时清空
└─ updates/          # 版本更新的下载和状态，可以随时清空（ADR 0014）
```

- 只有 `ferry-store` 知道这些路径，其他 crate 通过它访问。
- 配置结构变化必须递增 `schema_version` 并提供迁移，不得静默改变已有字段的含义。

## 11. 冻结与扩展

| 冻结，不可更改 | 可以扩展 |
|---|---|
| 顶层目录及用途 | 按规则新增 crate |
| 分层与依赖规则 | crate 内部的模块划分 |
| 命名规则 | `docs/` 下新增文档 |
| crate 内部结构与测试布局 | `assets/` 下新增子目录 |
| 数据目录位置与 `schema_version` 迁移机制 | 配置字段（通过迁移） |

## 12. 提交前检查

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask lint
cargo deny check

# 改动 app/ 时另外运行（在 app/ 目录下）
dart format --set-exit-if-changed lib test
flutter analyze
flutter test
```

以上检查全部通过才能提交。`cargo xtask lint` 自动检查第 4–9 节中可以静态验证的规则；新增 crate 时，先在 `xtask/src/deps.rs` 的 `layer()` 中登记层级。

## 13. 发布

决策见 ADR 0014。

### 版本号

`x.y.z`，现在是 0.x：

| 这一版有什么 | 升哪一位 | 例子 |
|---|---|---|
| 只有修复 | 最后一位 | 0.2.0 → 0.2.1 |
| 新功能、改进 | 中间一位，最后一位归零 | 0.2.3 → 0.3.0 |

没有单独的协议版本：两台之间的通信靠能力位协商，任意两个已发布的版本都要能互连（规则见 `docs/protocol.md` 第 7 节）。
新增消息必须配新的能力位，并且对方没有这个能力时有退路；不要改 `ferry_proto::WIRE_FORMAT`。

Cargo.toml 的 `[workspace.package] version` 是唯一的版本号，`app/pubspec.yaml` 跟着它走，都由 `cargo xtask release` 改，不要手改。

### 更新说明

每个版本一份 `docs/releases/<版本>.md`，给用户看：

```markdown
# ScreenFerry 0.2.0

## 简体中文

### 新增
- Mac 安装盘：打开后把应用拖进“应用程序”

### 改进
- 设置界面在 Windows 上只用一个窗口打开

### 修复
- 修好 Windows 上偶尔卡键

## English

### New
- Mac installer disk: open it and drag the app into Applications

### Improved
- The settings window opens as a single window on Windows

### Fixed
- Fixed keys occasionally getting stuck on Windows
```

- 中英文两节都要有，内容对应。分类只能是 `新增 / 改进 / 修复`（`New / Improved / Fixed`），按这个顺序写，没有内容的分类整个不写。
- 每条一行、以“- ”开头，写用户看得到的变化，不写实现细节、crate 名、ADR 编号。
- 它是 GitHub 发布页的正文，也写进更新清单，程序的更新卡片按分类显示。
- CHANGELOG.md 是给开发者看的，只写中文，可以写细；两者不互相替代。

### 发布产物

每个版本的附件正好 5 个，名字是 `ScreenFerry-<版本>-<系统>-<架构>.<扩展名>`：

| 文件 | 用途 |
|---|---|
| `ScreenFerry-<版本>-macos-universal.dmg` | Mac 第一次安装 |
| `ScreenFerry-<版本>-macos-universal.zip` | Mac 自动更新 |
| `ScreenFerry-<版本>-windows-x64-setup.exe` | Windows 安装和自动更新 |
| `update.json`、`update.json.minisig` | 程序检查更新时读，名字不能改 |

- 不另附 `.sha256` 之类的校验文件：每个包的大小和 SHA-256 写在签名过的 `update.json` 里。
- macOS 的包都是通用版（arm64 + x86_64），签名并公证；Windows 安装包不签名（ADR 0011）。
- 附件只由 CI 生成，不手动上传、不替换已发布版本的附件。

### 流程

1. 写好 `docs/releases/<版本>.md`，CHANGELOG 的 `## [Unreleased]` 下记全这一版的改动。
2. `cargo xtask release <版本>`：检查更新说明，改版本号，把 CHANGELOG 的 `[Unreleased]` 归到这个版本，提交 `release: <版本>` 并打 tag `v<版本>`。
3. `git push && git push origin v<版本>`：CI（`release.yml`）构建两个系统的产物，写好签名的更新清单，存成草稿。
4. 在 GitHub 上看过草稿（附件是不是 5 个、更新说明对不对），再点“公开”。公开之后，程序的自动更新才看得到它。

- 只发正式版，不发测试版、不把草稿标成 pre-release。
- 发布私钥、证书、公证密钥只放在 GitHub 的 Repository secrets 里（名字见 `release.yml` 开头），不进仓库、不进日志。
- 已经公开的版本有问题时，发一个新的修复版，不删除、不改动已公开的版本。
