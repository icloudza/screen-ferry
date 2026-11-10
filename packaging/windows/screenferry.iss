; ScreenFerry 的 Windows 安装包，英文和简体中文两种语言。需要 Inno Setup 7.1 或更高版本（从这一版起自带简体中文）。
; 由 `cargo xtask bundle` 编译，它传入：
;   AppVersion  版本号，取自 Cargo.toml
;   SourceDir   组装好的程序目录：screenferry.exe 和 settings\
; 并用 --output-dir 指定安装包放在哪里。设计见 docs/adr/0011-windows-installer.md。

#ifndef AppVersion
  #error 请用 cargo xtask bundle 编译（缺少 AppVersion）
#endif
#ifndef SourceDir
  #error 请用 cargo xtask bundle 编译（缺少 SourceDir）
#endif

#define AppName "ScreenFerry"
#define AppExe "screenferry.exe"
#define SettingsExe "screenferry-settings.exe"
#define ServiceExe "screenferry-service.exe"
#define Repo "https://github.com/icloudza/screen-ferry"

[Setup]
; 安装包的身份：升级、卸载都靠它认出已经装好的那一份，不能改。
AppId={{CEE535CE-67B9-466B-883F-0E39783CFB8C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=icloudza
AppPublisherURL={#Repo}
AppSupportURL={#Repo}/issues
AppUpdatesURL={#Repo}/releases
AppCopyright=MIT OR Apache-2.0
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppName} Setup
VersionInfoProductName={#AppName}
; 装进 Program Files，这台电脑上的所有用户都能用；要管理员权限，同时登记防火墙规则。
PrivilegesRequired=admin
SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
MinVersion=10.0
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
WizardStyle=modern
SetupIconFile=..\..\assets\icons\app-icon-windows.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/max
SolidCompression=yes
OutputBaseFilename={#AppName}-{#AppVersion}-windows-x64-setup
SetupLogging=yes
; 程序文件被占用时：安装前先请 ScreenFerry 自己退出（见 [Code]），剩下的交给系统的重启管理器关闭。
CloseApplications=yes
RestartApplications=no
; 开机自启登记在当前用户名下（HKCU）：安装时改成新的位置，卸载时删掉（见 [Code]）。
UsedUserAreasWarning=no
UsePreviousTasks=yes
; 按 Windows 的显示语言选：中文系统用简体中文，其他都用英文（排在第一个的语言），不弹出选择语言的对话框。
LanguageDetectionMethod=uilanguage
ShowLanguageDialog=no

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "chs"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[CustomMessages]
en.AppComment=Share one keyboard and mouse across your computers
chs.AppComment=在多台电脑之间共享一套键盘鼠标
en.FirewallStatus=Configuring Windows Firewall…
chs.FirewallStatus=正在设置 Windows 防火墙…
en.SecureInput=Allow paired computers to control Windows permission prompts (UAC)
chs.SecureInput=允许已配对的电脑操作 Windows 权限提示（UAC）
en.ServiceStatus=Configuring secure input…
chs.ServiceStatus=正在设置安全桌面输入…
en.ServiceStopFailed=Unable to stop secure input. Close ScreenFerry and try again.
chs.ServiceStopFailed=无法停止安全输入。请关闭 ScreenFerry 后重试。
en.ServiceConfigureFailed=Unable to configure secure input. Use the default installation folder or leave secure input unchecked and try again.
chs.ServiceConfigureFailed=无法设置安全输入。请使用默认安装位置，或取消安全输入选项后重试。

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
; 默认不选；升级时沿用上次的选择（UsePreviousTasks），不会自动增加权限（ADR 0015）。
Name: "secureinput"; Description: "{cm:SecureInput}"; Flags: unchecked

[InstallDelete]
; 设置界面整个换新，不留旧版本的资源文件。
Type: filesandordirs; Name: "{app}\settings"

[Files]
Source: "{#SourceDir}\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\{#ServiceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\settings\*"; DestDir: "{app}\settings"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; Comment: "{cm:AppComment}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; 防火墙：允许本程序在专用网络和域网络上接受连接（键鼠、剪贴板、邀请、局域网发现），先删后加，重装不会重复。
; 公用网络不放行，连到公用网络时 Windows 会照常询问。
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=""{#AppName}"""; Flags: runhidden waituntilterminated; StatusMsg: "{cm:FirewallStatus}"
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall add rule name=""{#AppName}"" dir=in action=allow program=""{app}\{#AppExe}"" enable=yes profile=private,domain"; Flags: runhidden waituntilterminated
; 安装前正在运行的：装完马上以原来的用户身份重新启动，不弹出设置界面。
; 用户可能正从另一台电脑控制这台，这样不用等到点“完成”就能接着用键鼠。
Filename: "{app}\{#AppExe}"; Parameters: "--autostart"; Flags: nowait runasoriginaluser; Check: WasRunning
; 第一次安装：完成页上可以选择启动，打开设置界面开始配对。
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent runasoriginaluser; Check: not WasRunning

[UninstallRun]
; 先请 ScreenFerry 正常退出（归还键鼠、松开按键），还在的再强行结束，然后删掉防火墙规则。
Filename: "{app}\{#AppExe}"; Parameters: "quit"; Flags: runhidden waituntilterminated; RunOnceId: "QuitApp"
Filename: "{app}\{#ServiceExe}"; Parameters: "remove"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveInputService"; Check: HasInputService
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#AppExe} /IM {#SettingsExe}"; Flags: runhidden waituntilterminated; RunOnceId: "KillApp"
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=""{#AppName}"""; Flags: runhidden waituntilterminated; RunOnceId: "RemoveFirewallRule"

[Code]
const
  RunKey = 'Software\Microsoft\Windows\CurrentVersion\Run';

var
  Running: Boolean;

{ 安装前 ScreenFerry 是否在运行：它运行时在缓存目录里写着 ipc.json，退出时删掉。 }
function WasRunning: Boolean;
begin
  Result := Running;
end;

function SecureInputSelected: Boolean;
begin
  Result := WizardIsTaskSelected('secureinput');
end;

function HasInputService: Boolean;
var
  Command: String;
begin
  Result := RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Services\ScreenFerryInput', 'ImagePath', Command)
    and (Pos(Lowercase(ExpandConstant('{app}\{#ServiceExe}')), Lowercase(Command)) > 0);
end;

{ 安装前请正在运行的 ScreenFerry 退出，程序文件才能替换：
  先用新版本的 quit 命令请它正常退出（归还键鼠、松开按键，设置界面跟着关闭），
  不支持 quit 的旧版本再强行结束。这样也不会弹出“关闭应用程序”的页面。 }
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Running := FileExists(ExpandConstant('{localappdata}\{#AppName}\ipc.json'));
  if Running then
  begin
    ExtractTemporaryFile('{#AppExe}');
    Exec(ExpandConstant('{tmp}\{#AppExe}'), 'quit', '', SW_HIDE, ewWaitUntilTerminated, Code);
    Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe} /IM {#SettingsExe} /IM screenferry_settings.exe',
      '', SW_HIDE, ewWaitUntilTerminated, Code);
  end;
  { 没选安全输入、也从没装过这个服务的，下面两步都跳过：安装过程和以前完全一样，不运行服务程序。 }
  if HasInputService or SecureInputSelected then
    ExtractTemporaryFile('{#ServiceExe}');
  { 停止旧服务及其 Job 中的代理，之后才能替换服务程序。
    必须运行新安装包里的工具，不能以管理员身份执行目标目录中可能被伪造的旧程序。 }
  if HasInputService then
  begin
    if not Exec(ExpandConstant('{tmp}\{#ServiceExe}'), 'stop', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
    begin
      Result := CustomMessage('ServiceStopFailed');
      Exit;
    end;
  end;
  { 在复制文件前验证/创建受保护的应用目录。 }
  if SecureInputSelected then
  begin
    if not Exec(ExpandConstant('{tmp}\{#ServiceExe}'), 'prepare "' + ExpandConstant('{app}') + '"', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
    begin
      Result := CustomMessage('ServiceConfigureFailed');
      Exit;
    end;
  end;
  Result := '';
end;

{ 开着开机自启时，登记的可能还是旧的位置（例如解压版的文件夹），改成安装后的位置。 }
procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    if RegValueExists(HKCU, RunKey, '{#AppName}') then
      RegWriteStringValue(HKCU, RunKey, '{#AppName}', '"' + ExpandConstant('{app}\{#AppExe}') + '" --autostart');
    { 服务这一步失败不让整个安装失败：键鼠共享、剪贴板和文件照常能用，只是权限提示还得在这台电脑上自己点。 }
    if SecureInputSelected then
    begin
      if not Exec(ExpandConstant('{app}\{#ServiceExe}'), 'install', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
      begin
        Log('安全输入服务没有装上，退出码 ' + IntToStr(Code));
        SuppressibleMsgBox(CustomMessage('ServiceConfigureFailed'), mbError, MB_OK, IDOK);
      end;
    end
    else if HasInputService then
    begin
      { 这次没选：去掉以前装的服务和它的授权。 }
      if not Exec(ExpandConstant('{tmp}\{#ServiceExe}'), 'remove', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
        Log('安全输入服务没有移除干净，退出码 ' + IntToStr(Code));
    end;
  end;
end;

{ 卸载时删掉指向本程序的开机自启登记，登录时不再去找已经删掉的程序。 }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Command: String;
begin
  if (CurUninstallStep = usUninstall)
    and RegQueryStringValue(HKCU, RunKey, '{#AppName}', Command)
    and (Pos(Lowercase(ExpandConstant('{app}')), Lowercase(Command)) > 0) then
    RegDeleteValue(HKCU, RunKey, '{#AppName}');
end;
