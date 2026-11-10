# Actions 安装包验收

这些是同一次提交的 release 构建，包含完整设置界面；通过 Actions Artifacts 下载，尚未发布 GitHub Release。
版本号暂保留工作区的 0.1.0，线协议已升为 2。两端必须从同一次 Validation Installers 构建下载，不要混用已发布的协议 1 安装包。
Artifacts 保留 14 天，名称中的提交号用于区分构建。

## 安装和普通输入

1. 下载 macOS 和 Windows 对应 Artifact 并解压。macOS 打开其中的 DMG，把 ScreenFerry 拖入“应用程序”；Windows 运行其中的 setup.exe。
2. Windows 安装器里的 UAC 安全输入选项默认不选。需要验收 UAC 时明确勾选。
3. 打开两端设置界面，Mac 选控制、Windows 选被控制。配对、安排屏幕后检查移动、点击、滚轮、输入及双向边缘停留时间。
4. 关闭两端设置窗口，检查后台共享继续工作。Mac 按 Ctrl + Option + Esc 应立即归还控制。

## 单独授权 UAC 输入

普通配对不授予 UAC 权限。Mac 在终端运行以下命令，只复制输出的完整公开授权码：

```bash
/Applications/ScreenFerry.app/Contents/MacOS/screenferry input-key
```

Windows 在普通用户 PowerShell 运行 `whoami /user`，记下真正运行后台的用户 SID。
在管理员 PowerShell 执行：

```powershell
& "$env:ProgramFiles\ScreenFerry\screenferry-service.exe" authorize "<普通用户SID>" "<Mac公开授权码>"
```

在 Windows 设置界面暂停再恢复共享，重新建立连接。若选择了自定义安装位置，相应调整服务程序路径。
然后从 Windows 的**普通用户** PowerShell 触发权限提示：

```powershell
Start-Process notepad.exe -Verb RunAs
```

从 Mac 分别测试取消和确认；确认后检查管理员记事本输入。还需验证锁屏恢复、按住修饰键/鼠标键时切换桌面、断网、停止后台和停止服务后的状态清理。
签名验证输入来源，当前普通后台仍可见输入正文，不提供防后台读取的端到端保密；不要用此构建输入真实管理员密码。

撤销授权：

```powershell
& "$env:ProgramFiles\ScreenFerry\screenferry-service.exe" revoke "<普通用户SID>" "<完整设备指纹>"
```

完整设备指纹是公开授权码冒号前的 64 字符。检查当前安全输入会话结束、重新连接后仍能普通权限共享，但不能控制 UAC。
卸载服务会清除机器级输入授权，保留用户配对记录。

## 反馈

记录 Actions 运行链接/提交号、Windows/macOS 版本、每一步的通过或失败及同一时间段的两端诊断日志。
完整的安全边界和验收清单见仓库 `docs/input-troubleshooting.md`。本次打包完成不代表 UAC 或锁屏已经通过实机验收。
