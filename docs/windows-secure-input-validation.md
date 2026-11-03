# Windows 安全输入手工验收

本包是命令行实验验收包，不含 Flutter 设置界面，不是正式安装器。仅在没有现有 ScreenFerry 安装的隔离 Windows 10/11 x64 测试机使用，使用测试账户和测试密码。
输入来源认证不提供防普通后台读取输入正文的端到端保密；不要用本包输入真实管理员密码。

## 1. 准备两端

生成本包的命令：`cargo xtask input-validation`（Windows/MSVC），或在配置好交叉工具链的 Mac 上：

```text
cargo xtask input-validation --target x86_64-pc-windows-gnu
```

输出在 `target/input-validation/windows/`。`ScreenFerry/` 里只有后台和服务两个程序；`build.json` 记录构建目标、实验开关和各程序的 SHA-256。
macOS/Windows 还会生成 `target/input-validation/ScreenFerry-secure-input-experimental-windows-x64.zip`，解压后进入包含 README 的 `windows/` 目录。
这是 release 构建的独立测试产物，不是 GitHub Release 附件，也不能用于验证正式安装器的安装/更新流程。

Mac 也必须使用同一开发代码（协议 2），不要使用协议 1 的已发布程序。退出原来的后台，然后在 Mac 的仓库目录运行：

```text
cargo run -p ferry-daemon -- serve --no-announce --no-clipboard -v
```

若提示没有捕获权限，按 README 授予当前开发程序辅助功能权限。记下 Mac 的局域网 IP。

## 2. 在 Windows 安装实验服务

把包复制到 Windows 并解压，在解压目录打开**管理员 PowerShell**，执行：

```powershell
.\ScreenFerry\screenferry-service.exe prepare "C:\Program Files\ScreenFerry"
Copy-Item .\ScreenFerry\*.exe "C:\Program Files\ScreenFerry\"
& "$env:ProgramFiles\ScreenFerry\screenferry-service.exe" install
Get-Service ScreenFerryInput
```

预期服务为 Running。如果任何命令失败，先记录错误，不要改宽安装目录或注册表 ACL 来绕过检查。

在 Windows 打开**普通用户 PowerShell**：

```powershell
whoami /user
& "$env:ProgramFiles\ScreenFerry\screenferry.exe" connect <Mac的IP> --no-clipboard -v
```

两端核对配对码并确认。此时只有普通配对权限：先验证记事本输入、鼠标和返回 Mac 正常；此时不能操作 UAC。

## 3. 单独授权控制端

在 Mac 另开终端，在同一仓库运行 `cargo run -p ferry-daemon -- input-key`，取得 195 字符公开授权码；通过可信渠道核对完整输出。
在 Windows 管理员 PowerShell 执行（SID 使用上面普通用户的 SID）：

```powershell
& "$env:ProgramFiles\ScreenFerry\screenferry-service.exe" authorize "<用户SID>" "<控制端公开授权码>"
```

Windows 普通用户终端按 Ctrl+C 结束 connect，再运行一次 connect 命令，重新协商签署输入。

## 4. 验证并记录结果

逐项记录“通过 / 失败 + 当时日志”，不要提前勾选：

1. 普通窗口输入、点击、滚轮、边缘返回正常。
2. Windows 普通终端运行 `Start-Process notepad.exe -Verb RunAs`，分别测试从 Mac 操作 UAC 的取消和确认；确认后可在管理员记事本输入。
3. 按住 Shift/Ctrl/鼠标键时出现 UAC，随后松开；回到普通桌面无卡键、拖动或残留隐藏光标。
4. 锁屏后用测试账户解锁，检查输入和切换恢复；Default/Winlogon 多次往返后无持续增长的线程、句柄或内存。
5. 分别断网、停止 Windows connect、停止服务，检查 Mac 收回控制、Windows 无残留按键和隐藏光标。停止服务命令为管理员运行 `screenferry-service stop`。
6. 管理员执行 `screenferry-service revoke "<用户SID>" "<设备完整指纹>"`，检查当前安全输入会话结束，之后普通权限共享仍可连接，UAC 控制被拒绝。完整指纹是公开授权码冒号前的 64 字符。
7. 重启 Windows 后验证服务、重新连接及授权状态；不要把普通配对成功误记为 UAC 授权成功。

## 5. 清理和反馈

结束两端命令行程序。在 Windows 管理员 PowerShell 执行：

```powershell
& "$env:ProgramFiles\ScreenFerry\screenferry-service.exe" remove
Get-Service ScreenFerryInput -ErrorAction SilentlyContinue
```

预期服务登记和机器级输入授权都移除，普通用户的配对配置保留。若曾手工删除过服务登记，也运行 remove 清理授权。

反馈请带：`build.json`、Windows 版本、失败的步骤、UAC 是确认还是凭据提示、两端同一时间段的日志，以及是否存在残留按键/光标。
不要提供密码、私钥或实际输入的敏感内容。完整安装器、冒充管道及权限异常验收仍见 `docs/input-troubleshooting.md`，本包没有替代这些检查。
