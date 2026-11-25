// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get appTitle => 'ScreenFerry';

  @override
  String get commonSave => '保存';

  @override
  String get commonCancel => '取消';

  @override
  String get commonClose => '关闭';

  @override
  String get navOverview => '仪表盘';

  @override
  String get navDevices => '设备';

  @override
  String get navLayout => '布局';

  @override
  String get navLog => '日志';

  @override
  String get navSettings => '设置';

  @override
  String get sidebarExpand => '展开侧栏';

  @override
  String get sidebarCollapse => '收起侧栏';

  @override
  String get statusConnectingDaemon => '正在连接后台';

  @override
  String get statusDaemonNotRunning => '后台未运行';

  @override
  String get statusDaemonLost => '与后台断开';

  @override
  String get statusSharing => '共享中';

  @override
  String statusSharingRtt(String ms) {
    return '共享中 · $ms ms';
  }

  @override
  String get statusPaused => '已暂停共享';

  @override
  String get statusWaiting => '等待对方连接';

  @override
  String get statusSearching => '正在寻找对方';

  @override
  String get statusConnecting => '正在连接';

  @override
  String get statusStarting => '正在启动';

  @override
  String get statusStopped => '已停止';

  @override
  String get windowMinimize => '最小化';

  @override
  String get windowRestore => '还原';

  @override
  String get windowMaximize => '最大化';

  @override
  String get chartSwitchesLabel => '今日各小时的切换次数';

  @override
  String get chartMidnight => '0 时';

  @override
  String get chartNoon => '12 时';

  @override
  String get chartNow => '现在';

  @override
  String get bannerDaemonLost => '与后台的连接已断开，正在重连…';

  @override
  String get bannerPermission => '需要键鼠控制权限';

  @override
  String get bannerPortInUse => '端口被占用';

  @override
  String get bannerOther => '后台遇到了问题';

  @override
  String get bannerOpenSystemSettings => '打开系统设置';

  @override
  String get problemPermission => '需要在系统设置的“隐私与安全性”中允许 ScreenFerry 控制键盘和鼠标';

  @override
  String problemPortInUse(int port) {
    return '端口 $port 被占用：可能还开着另一个键鼠共享软件或另一份 ScreenFerry';
  }

  @override
  String get offlineConnecting => '正在连接后台…';

  @override
  String get offlineNotRunning => '后台没有运行';

  @override
  String get offlineStartHint => '请先启动 ScreenFerry，连上后会自动显示。';

  @override
  String get offlineAutoConnect => '启动后会自动连上。';

  @override
  String get offlineStartFailed => '启动失败，请在“应用程序”中打开 ScreenFerry';

  @override
  String get offlineStarting => '正在启动…';

  @override
  String get offlineStart => '启动 ScreenFerry';

  @override
  String get offlineRetrying => '正在重试';

  @override
  String get nameEditorRename => '改名字';

  @override
  String get accentBlue => '蓝色';

  @override
  String get accentTeal => '青色';

  @override
  String get accentPurple => '紫色';

  @override
  String get accentOrange => '橙色';

  @override
  String get accentPink => '粉色';

  @override
  String get aboutTitle => '关于';

  @override
  String get aboutBack => '返回设置';

  @override
  String get aboutTagline => '在多台电脑之间共享一套键盘鼠标';

  @override
  String get aboutCheckUpdates => '检查更新';

  @override
  String get aboutUpdatesSoon => '自动检查更新即将支持';

  @override
  String get aboutAuthor => '作者主页';

  @override
  String get aboutLicense => '许可证';

  @override
  String get logTitle => '日志';

  @override
  String get logClear => '清空';

  @override
  String get logFilter => '筛选';

  @override
  String get logAll => '全部';

  @override
  String get logCursor => '光标';

  @override
  String get logClipboard => '剪贴板';

  @override
  String get logFiles => '文件';

  @override
  String get logLinks => '设备与连接';

  @override
  String get logRun => '运行日志';

  @override
  String get logEmpty => '暂无记录';

  @override
  String logCount(int count) {
    return '共 $count 条';
  }

  @override
  String logRunSize(String size, String lines) {
    return '$size · $lines 行';
  }

  @override
  String get pairingTitle => '配对';

  @override
  String pairingWith(String name) {
    return '与“$name”配对';
  }

  @override
  String get pairingCheck => '请确认两台电脑上显示的是同一个配对码，一致才确认。';

  @override
  String get pairingPeerFingerprint => '对方指纹 ';

  @override
  String get pairingWaiting => '已确认，等待对方确认…';

  @override
  String pairingAutoReject(int seconds) {
    return '$seconds 秒后自动拒绝';
  }

  @override
  String get pairingTimedOut => '已超时';

  @override
  String get pairingReject => '拒绝';

  @override
  String get pairingAccept => '数字一致，配对';

  @override
  String pairingCodeLabel(String code) {
    return '配对码 $code';
  }

  @override
  String pairingDone(String name) {
    return '已与“$name”配对';
  }

  @override
  String get pairingNoMoreAsking => '以后连接不再询问';

  @override
  String dialBadAddressTitle(String address) {
    return '地址不对：$address';
  }

  @override
  String get dialBadAddressBody => '填 IP（例如 192.168.2.12）、IP:端口，或者电脑名。';

  @override
  String dialNotWaitingInviteTitle(String address) {
    return '$address 没有在等待邀请';
  }

  @override
  String get dialNotWaitingInviteBody => '确认那台电脑开着 ScreenFerry，并且选的是“被控制”。它的版本太旧时也不能被邀请，需要先升级。';

  @override
  String dialNotWaitingTitle(String address) {
    return '$address 没有在等待连接';
  }

  @override
  String get dialNotWaitingBody => '确认那台电脑开着 ScreenFerry，并且在设置里选的是“控制别的电脑”。';

  @override
  String dialUnreachableTitle(String address) {
    return '连不上 $address';
  }

  @override
  String get dialUnreachableBody => '确认两台在同一个网络里、地址没有填错。Windows 第一次运行 ScreenFerry 时会询问防火墙，请允许它通过。';

  @override
  String dialNotScreenFerryTitle(String address) {
    return '$address 上不是 ScreenFerry';
  }

  @override
  String get dialNotScreenFerryBody => '确认地址和端口没有填错。';

  @override
  String get dialIncompatibleTitle => '版本不兼容';

  @override
  String get dialIncompatibleBody => '两台电脑上的 ScreenFerry 版本相差太多，请都升级到最新版。';

  @override
  String dialBusyTitle(String address) {
    return '$address 正连着另一台电脑';
  }

  @override
  String get dialBusyBody => '先在那台电脑上断开，再邀请。';

  @override
  String get dialWrongRoleInviteTitle => '本机现在是被控制的一方';

  @override
  String get dialWrongRoleInviteBody => '邀请是控制端用的；被控制的一方请用“按地址连接”。';

  @override
  String get dialWrongRoleTitle => '本机现在是控制端';

  @override
  String get dialWrongRoleBody => '按地址连接是被控制的一方用的；控制端请用“邀请一台电脑”。';

  @override
  String get dialInviteTitle => '邀请一台电脑';

  @override
  String get dialConnectTitle => '按地址连接';

  @override
  String dialInvited(String address) {
    return '已邀请 $address，它正在连过来';
  }

  @override
  String dialConnected(String address) {
    return '已连上 $address';
  }

  @override
  String get dialInviteHint => '那台电脑要开着 ScreenFerry，选的是“被控制”。地址在它的「设备」页上；不写端口时用 24811。';

  @override
  String get dialConnectHint => '可以是 IP、IP:端口或电脑名，不写端口时用 24810。地址在那台电脑的「设备」页上。';

  @override
  String get dialInviteSubtitle => '输入被控制那台的地址，它会自动连过来';

  @override
  String get dialConnectSubtitle => '输入控制端（键盘鼠标所在那台）的地址';

  @override
  String get dialAddress => '地址';

  @override
  String dialExample(String address) {
    return '例如 $address';
  }

  @override
  String dialInviting(String address) {
    return '正在邀请 $address…';
  }

  @override
  String dialConnecting(String address) {
    return '正在连接 $address…';
  }

  @override
  String get dialRetry => '重试';

  @override
  String get dialInvite => '邀请';

  @override
  String get dialConnect => '连接';

  @override
  String get diagTitle => '提问题时附上诊断文件';

  @override
  String get diagBody => '包含版本、系统、设置和最近的运行日志。设备名、用户名和文件名会换成代号，不含剪贴板内容和密钥。';

  @override
  String get diagCopySummary => '复制摘要';

  @override
  String get diagExport => '导出诊断文件';

  @override
  String get diagOpenFolder => '打开日志文件夹';

  @override
  String get diagReport => '去 GitHub 提问题';

  @override
  String get diagSaved => '已存到“下载”：';

  @override
  String get diagShowInFinder => '在访达中显示';

  @override
  String get diagShowInExplorer => '在资源管理器中显示';

  @override
  String get diagVerbose => '详细记录';

  @override
  String get diagVerboseOff => '排查连接问题时打开，会多记一些细节。重启 ScreenFerry 后自动关闭';

  @override
  String get diagVerboseJustOn => '刚刚打开。问题出现后导出，再把它关掉';

  @override
  String diagVerboseOnFor(int minutes) {
    return '已打开 $minutes 分钟。问题出现后导出，再把它关掉';
  }

  @override
  String get diagRecent => '最近的运行日志';

  @override
  String get diagLive => '实时';

  @override
  String get diagNotConnected => '未连接';

  @override
  String get diagLevel => '日志级别';

  @override
  String get diagProblemsOnly => '警告和错误';

  @override
  String get diagNoProblems => '最近没有警告和错误';

  @override
  String get diagNoLog => '还没有运行日志';

  @override
  String get diagLevelDebug => '调试';

  @override
  String get diagLevelInfo => '信息';

  @override
  String get diagLevelWarn => '警告';

  @override
  String get diagLevelError => '错误';

  @override
  String get diagCopied => '已复制摘要：版本、系统和最近 200 行，可以直接贴进问题描述';

  @override
  String get keyMapTitle => '添加按键映射';

  @override
  String get keyMapNothing => '不发送';

  @override
  String keyMapOnlyFor(String name) {
    return '只对 $name 生效';
  }

  @override
  String get keyMapStep1 => '① 本机按哪个键';

  @override
  String get keyMapRecording => '正在录制，按一个键';

  @override
  String get keyMapRerecord => '重新录制';

  @override
  String get keyMapPressAKey => '请在键盘上按一个键…';

  @override
  String get keyMapEscCancels => 'Esc 取消';

  @override
  String get keyMapRecorded => '已录下';

  @override
  String get keyMapClickToRecord => '点这里，再按一个键';

  @override
  String get keyMapModifiersToo => '也能录单个修饰键，左右分开算。';

  @override
  String keyMapStep2(String name) {
    return '② $name 收到什么';
  }

  @override
  String get keyMapSearch => '搜索按键，例如 Home、F5、音量';

  @override
  String get keyMapPress => '按';

  @override
  String keyMapGetsNothing(String name) {
    return '$name 什么也收不到';
  }

  @override
  String keyMapGets(String name) {
    return '$name 收到';
  }

  @override
  String get keyMapAdd => '添加';

  @override
  String get sideLeft => '左侧';

  @override
  String get sideRight => '右侧';

  @override
  String get sideTop => '上方';

  @override
  String get sideBottom => '下方';

  @override
  String get peerFallback => '对方';

  @override
  String get toastLinkCopied => '打不开浏览器，链接已复制';

  @override
  String get toastNotConnected => '还没连上后台，稍后再试';

  @override
  String toastConnectingTo(String name) {
    return '正在连接 $name…';
  }

  @override
  String get timeJustNow => '刚刚';

  @override
  String timeSecondsAgo(int n) {
    return '$n 秒前';
  }

  @override
  String timeMinutesAgo(int n) {
    return '$n 分钟前';
  }

  @override
  String timeHoursAgo(int n) {
    return '$n 小时前';
  }

  @override
  String timeDaysAgo(int n) {
    return '$n 天前';
  }

  @override
  String get seenOnline => '在线';

  @override
  String get seenOffline => '离线';

  @override
  String seenAgo(String ago) {
    return '$ago连接过';
  }

  @override
  String get durationUnderMinute => '不到 1 分钟';

  @override
  String durationMinutes(int m) {
    return '$m 分钟';
  }

  @override
  String durationHours(int h, int m) {
    return '$h 小时 $m 分';
  }

  @override
  String keyMedia(String hex) {
    return '媒体键 0x$hex';
  }

  @override
  String keyUnknown(String hex) {
    return '按键 0x$hex';
  }

  @override
  String keyNumpad(String key) {
    return '小键盘 $key';
  }

  @override
  String keyLeft(String key) {
    return '左 $key';
  }

  @override
  String keyRight(String key) {
    return '右 $key';
  }

  @override
  String get keySpace => '空格';

  @override
  String get keyIso => '\\（ISO）';

  @override
  String get keyMenu => '菜单键';

  @override
  String get keyMute => '静音';

  @override
  String get keyVolumeUp => '音量 +';

  @override
  String get keyVolumeDown => '音量 −';

  @override
  String get keyHangul => '韩/英、かな';

  @override
  String get keyHanja => '汉字、英数';

  @override
  String get keyForwardDelete => '⌦ 向前删除';

  @override
  String get keyBrightnessUp => '亮度 +';

  @override
  String get keyBrightnessDown => '亮度 −';

  @override
  String get keyNextTrack => '下一首';

  @override
  String get keyPreviousTrack => '上一首';

  @override
  String get keyStop => '停止';

  @override
  String get keyPlayPause => '播放/暂停';

  @override
  String get settingsThisComputer => '这台电脑';

  @override
  String get settingsRole => '角色';

  @override
  String get settingsRoleServerHint => '键盘鼠标接在这台上，用它控制别的电脑';

  @override
  String get settingsRoleClientHint => '由另一台电脑的键盘鼠标控制 · 切换后重新连接';

  @override
  String get settingsRoleServer => '控制别的电脑';

  @override
  String get settingsRoleClient => '被控制';

  @override
  String get settingsAppearance => '外观';

  @override
  String get settingsTheme => '主题';

  @override
  String get settingsFollowSystem => '跟随系统';

  @override
  String get settingsLight => '浅色';

  @override
  String get settingsDark => '深色';

  @override
  String get settingsLanguage => '语言';

  @override
  String get settingsAccent => '强调色';

  @override
  String get settingsAccentHint => '开关、选中项和按钮的颜色';

  @override
  String get settingsMouseClipboard => '鼠标与剪贴板';

  @override
  String get settingsDpi => '本机鼠标的 DPI';

  @override
  String get settingsDpiHint => '每台设备的光标速度以它为准';

  @override
  String get settingsPerDevice => '按键映射、滚动方向';

  @override
  String get settingsPerDeviceHint => '在“设备”里逐台设置';

  @override
  String get settingsClipboard => '同步剪贴板';

  @override
  String get settingsClipboardHint => '光标切过去时，对方才来取最新的内容';

  @override
  String get settingsClipLimit => '单项大小上限';

  @override
  String get settingsClipLimitHint => '图片、文字等每一项超过这么大就不同步';

  @override
  String get settingsGeneral => '通用';

  @override
  String get settingsAutostart => '开机自动启动';

  @override
  String get settingsAnnounce => '在局域网里广播本机';

  @override
  String get settingsAnnounceHint => '另一台电脑不用填地址就能找到本机';

  @override
  String get settingsDataDir => '数据目录';

  @override
  String get settingsAbout => '关于 ScreenFerry';

  @override
  String settingsVersion(String version) {
    return '版本 $version';
  }

  @override
  String get commonComma => '，';

  @override
  String get commonEnum => '、';

  @override
  String get commonConnected => '已连接';

  @override
  String get commonNotConnected => '未连接';

  @override
  String get commonThisComputer => '本机';

  @override
  String get layoutTitle => '布局';

  @override
  String layoutSetOnServer(String name) {
    return '屏幕位置在 $name 上设置';
  }

  @override
  String get layoutSetOnServerHint => '键盘鼠标在那台电脑上，由它决定从哪条边切过来。';

  @override
  String get layoutSwitchMode => '切换方式';

  @override
  String get layoutSwitchModeHint => '停留一下再切换，可以避免鼠标甩到边缘时误切 · 即将支持';

  @override
  String get layoutInstant => '立即';

  @override
  String get layoutDwell => '停留 0.15 秒';

  @override
  String get layoutNoDragSwitch => '拖拽时不切换';

  @override
  String get layoutNoDragSwitchHint => '按住鼠标键时，光标不会切到另一台电脑 · 目前始终开启';

  @override
  String get layoutBackHere => '回到本机';

  @override
  String get layoutBackHereHint => '光标卡在对方屏幕上时，按它立即回来';

  @override
  String get layoutGoPair => '去配对';

  @override
  String layoutWhere(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {'left': '左侧', 'right': '右侧', 'top': '上方', 'other': '下方'});
    return '本机$_temp0';
  }

  @override
  String layoutWhereNth(String side, int n) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': '左侧从上往下',
      'right': '右侧从上往下',
      'top': '上方从左往右',
      'other': '下方从左往右',
    });
    return '本机$_temp0第 $n 台';
  }

  @override
  String layoutPeerAt(String name, String where) {
    return '$name，在$where';
  }

  @override
  String get layoutCursorOnIt => '光标在这台上';

  @override
  String get layoutMovable => '可以拖动';

  @override
  String get layoutEmpty => '还没有配对的设备。在另一台电脑上运行 ScreenFerry 并配对后，它会出现在本机旁边';

  @override
  String layoutDropAt(String where) {
    return '松手后放到$where';
  }

  @override
  String layoutSingle(String name, String where) {
    return '$name 在$where，光标从相接的那条边穿过去';
  }

  @override
  String layoutSideCount(String side, int n) {
    String _temp0 = intl.Intl.selectLogic(side, {'left': '左侧', 'right': '右侧', 'top': '上方', 'other': '下方'});
    return '$_temp0 $n 台';
  }

  @override
  String layoutMany(String parts) {
    return '本机$parts。同一侧已连接的几台按先后平分那条边，光标从哪一段穿过去就到哪台';
  }

  @override
  String get layoutCursorHere => '光标在这里';

  @override
  String get devicesThisName => '本机的名字';

  @override
  String get devicesComputerName => '电脑名';

  @override
  String get devicesNameHint => '别的电脑上看到的就是这个名字。留空就用电脑名。';

  @override
  String get devicesThisFingerprint => '本机 · 指纹 ';

  @override
  String get devicesCopyFingerprint => '复制指纹';

  @override
  String get devicesFingerprintCopied => '指纹已复制';

  @override
  String get devicesPaired => '已配对';

  @override
  String devicesCount(int n) {
    return '$n 台';
  }

  @override
  String get devicesNonePaired => '还没有配对的设备';

  @override
  String get devicesNonePairedHint => '在另一台电脑上运行 ScreenFerry，两边核对配对码后会出现在这里。';

  @override
  String get devicesFound => '在局域网里发现';

  @override
  String get devicesNotPairedYet => '还没配对';

  @override
  String get devicesInvite => '邀请一台电脑';

  @override
  String get devicesConnectByAddress => '按地址连接';

  @override
  String get devicesWaiting => '等另一台电脑来找本机';

  @override
  String get devicesWaitingHint => '在另一台电脑上运行 ScreenFerry 并选择本机后，这里会弹出配对码请你核对。';

  @override
  String devicesWaitingHintAddress(String address) {
    return '它在局域网里找不到本机时，在那台电脑上按地址连接 $address；那台电脑没有键盘鼠标时，在这里邀请它连过来。';
  }

  @override
  String get devicesNoneFound => '局域网里没有找到其他设备';

  @override
  String get devicesNoneFoundHint => '有的网络不支持自动发现（例如公司网络、开了设备隔离的 Wi‑Fi）。可以按地址连接，地址在控制端的「设备」页上。';

  @override
  String get devicesThisAddress => '本机地址';

  @override
  String get devicesAddressCopied => '地址已复制';

  @override
  String devicesCopy(String address) {
    return '复制 $address';
  }

  @override
  String devicesCopyAddress(String address) {
    return '复制本机地址 $address';
  }

  @override
  String devicesOnlineAt(String address) {
    return '在线 · $address';
  }

  @override
  String devicesSettingsOf(String name) {
    return '$name 的设置';
  }

  @override
  String get devicesFingerprint => '指纹 ';

  @override
  String devicesPlace(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {'left': '左侧', 'right': '右侧', 'top': '上方', 'other': '下方'});
    return '在本机$_temp0';
  }

  @override
  String get devicesSettings => '设置';

  @override
  String get devicesNotPaired => '未配对';

  @override
  String devicesFingerprintShort(String short) {
    return '指纹 $short';
  }

  @override
  String get devicesAnnouncing => '正在广播';

  @override
  String get devicesPair => '配对';

  @override
  String get overviewTitle => '仪表盘';

  @override
  String get overviewSharing => '共享中';

  @override
  String get overviewPausedTitle => '已暂停共享';

  @override
  String get overviewPaused => '已暂停';

  @override
  String get overviewWaitingTitle => '等待对方连接';

  @override
  String get overviewWaiting => '等待连接';

  @override
  String get overviewSearchingTitle => '正在寻找对方';

  @override
  String get overviewSearching => '寻找中';

  @override
  String get overviewConnectingTitle => '正在连接…';

  @override
  String get overviewConnecting => '连接中';

  @override
  String get overviewStartingTitle => '正在启动…';

  @override
  String get overviewStarting => '启动中';

  @override
  String get overviewStoppedTitle => '暂时无法共享';

  @override
  String get overviewStopped => '已停止';

  @override
  String get overviewPausedHint => '键鼠留在本机。点右边的按钮继续共享';

  @override
  String get overviewAnnouncingHint => '已在局域网里广播本机，另一台电脑会自动找到这里';

  @override
  String get overviewNotAnnouncingHint => '没有广播本机，另一台电脑需要手动填写本机地址';

  @override
  String get overviewSearchingHint => '在局域网里寻找已配对的电脑';

  @override
  String get overviewConnectingHint => '找到了对方，正在建立加密连接';

  @override
  String get overviewStartingHint => '稍等片刻';

  @override
  String get overviewProblemHint => '后台遇到了问题';

  @override
  String get overviewStatus => '状态';

  @override
  String get overviewLatencyGreat => '延迟 优';

  @override
  String get overviewLatencyGood => '延迟 良好';

  @override
  String get overviewLatencyHigh => '延迟 较高';

  @override
  String overviewCursorOn(String name) {
    return '光标在 $name';
  }

  @override
  String get overviewCursorHere => '光标在本机';

  @override
  String overviewConnectedCount(int n) {
    return '已连接 $n 台';
  }

  @override
  String get overviewAddress => '地址';

  @override
  String get overviewEncryption => '加密';

  @override
  String get overviewFingerprint => '指纹';

  @override
  String get overviewPause => '暂停共享';

  @override
  String get overviewResume => '继续共享';

  @override
  String get overviewRtt => '往返延迟';

  @override
  String overviewPeak(String ms) {
    return '峰值 $ms';
  }

  @override
  String get overviewRttEmpty => '连上对方后显示';

  @override
  String overviewRttAverage(int n, String ms) {
    return '最近 $n 秒平均 $ms ms';
  }

  @override
  String get overviewClipboard => '剪贴板';

  @override
  String get overviewNoClip => '还没有同步过剪贴板';

  @override
  String get overviewNoClipHint => '复制后把光标切过去，对方就能直接粘贴。';

  @override
  String get overviewClipOffHint => '剪贴板同步已关闭，可以在设置里打开。';

  @override
  String overviewClipImage(int width, int height, String size) {
    return '图片 · $width×$height · $size';
  }

  @override
  String overviewClipRich(int n) {
    return '带格式的文本 · $n 字';
  }

  @override
  String overviewClipText(int n) {
    return '文本 · $n 字';
  }

  @override
  String get overviewAnImage => '一张图片';

  @override
  String get overviewNotText => '（不是文字）';

  @override
  String overviewClipImageLabel(int width, int height) {
    return '剪贴板图片 $width×$height';
  }

  @override
  String get overviewClipSync => '剪贴板同步';

  @override
  String get overviewAutostart => '开机自动启动';

  @override
  String get overviewAnnounce => '局域网广播';

  @override
  String get overviewOn => '已开启';

  @override
  String get overviewOff => '已关闭';

  @override
  String get overviewManage => '管理';

  @override
  String overviewFoundNew(int n) {
    return '局域网里发现 $n 台新设备';
  }

  @override
  String overviewThisName(String name) {
    return '$name · 本机';
  }

  @override
  String get overviewCopyFingerprint => '复制本机指纹';

  @override
  String get overviewToday => '今日';

  @override
  String overviewSwitches(int n) {
    return '次屏幕切换';
  }

  @override
  String overviewClips(int n) {
    return '次剪贴板同步';
  }

  @override
  String get overviewHours => '小时连接';

  @override
  String get deviceLater => '本机控制它时生效';

  @override
  String get deviceBack => '返回设备列表';

  @override
  String get deviceMouse => '鼠标';

  @override
  String get deviceInvertVertical => '垂直滚动反向';

  @override
  String get deviceInvertVerticalHint => '上下滚动的方向反过来';

  @override
  String get deviceInvertHorizontal => '水平滚动反向';

  @override
  String get deviceInvertHorizontalHint => '左右滚动的方向反过来';

  @override
  String get deviceScrollSpeed => '滚动速度';

  @override
  String get deviceScrollSpeedHint => '触控板的细小滚动到了这台上偏慢或偏快时调';

  @override
  String get deviceSlow => '慢';

  @override
  String get deviceNormal => '标准';

  @override
  String get deviceFast => '快';

  @override
  String get deviceNaturalTip => '本机开着“自然滚动”，在这台上也按自然方向滚。想和普通鼠标一样，打开“垂直滚动反向”。';

  @override
  String get deviceKeys => '按键';

  @override
  String get deviceModifiers => '修饰键';

  @override
  String get deviceModifierPreset => '修饰键方案';

  @override
  String get deviceMacHabit => 'Mac 习惯';

  @override
  String get deviceXkbHabit => 'PC / XKB 习惯';

  @override
  String get deviceKeyboardUnknown => '还不知道两台设备的键盘类型，修饰键先按原样发送。连上并交换系统信息后会自动调整。';

  @override
  String get deviceWindowsHabit => 'Windows 习惯';

  @override
  String get deviceAsIs => '原样';

  @override
  String get deviceCustom => '自定义';

  @override
  String get deviceOtherKeys => '其他按键映射';

  @override
  String get deviceOtherKeysHint => '把一个键换成另一个键，或者不发送';

  @override
  String get deviceTextMac => '文本编辑快捷键按 Mac 习惯';

  @override
  String get deviceTextXkb => '文本编辑快捷键按 PC / XKB 习惯';

  @override
  String get deviceTextWindows => '文本编辑快捷键按 Windows 习惯';

  @override
  String get deviceTextMacHint => '⌘← ⌘→ 到行首行尾，⌥← ⌥→ 按词移动，⌘⌫ 删到行首';

  @override
  String get deviceTextXkbHint => 'Home End 到行首行尾，Ctrl+← Ctrl+→ 按词移动';

  @override
  String get deviceTextWindowsHint => 'Home End 到行首行尾，Ctrl+← Ctrl+→ 按词移动';

  @override
  String get deviceTextShortcuts => '文本编辑快捷键';

  @override
  String get deviceSwitching => '切换与共享';

  @override
  String get deviceDwell => '防止误切过去';

  @override
  String get deviceDwellServerHint => '光标在边上停一会儿才切到这台';

  @override
  String get deviceDwellClientHint => '本机控制它时，光标在边上停一会儿才切过去';

  @override
  String get deviceDwellLabel => '切过去前在边上停多久';

  @override
  String get deviceOff => '关';

  @override
  String deviceSeconds(String s) {
    return '$s 秒';
  }

  @override
  String get deviceClipboardHint => '和这台之间同步复制的内容';

  @override
  String get deviceClipboardOffHint => '“设置”里的剪贴板总开关关着';

  @override
  String get deviceClipboardLabel => '和这台同步剪贴板';

  @override
  String get deviceDropFiles => '拖文件给它';

  @override
  String get deviceDropFilesHint => '把文件拖到朝向它的屏幕边上就发过去';

  @override
  String get deviceConnection => '连接';

  @override
  String get deviceThisDevice => '这台设备';

  @override
  String devicePresetMacDiffer(String name) {
    return '在 $name 上按 ⌘C、⌘V 就是复制、粘贴';
  }

  @override
  String devicePresetWinDiffer(String name) {
    return '在 $name 上按 Ctrl+C、Ctrl+V 就是复制、粘贴';
  }

  @override
  String get devicePresetSame => '两台是同一种键盘，原样发送';

  @override
  String get devicePresetPlainMac => '原样发送：⌘ 到了那边是 Win 键';

  @override
  String get devicePresetPlainWin => '原样发送：Win 键到了那边是 ⌘';

  @override
  String get devicePresetPlain => '原样发送';

  @override
  String get devicePresetCustom => '按下面的对照表逐个对应';

  @override
  String get deviceAlias => '在本机上叫它';

  @override
  String get deviceAliasHint => '只改本机上显示的名字，那台电脑自己的名字不变。留空就用原名。';

  @override
  String deviceOriginalName(String name) {
    return '原名 $name';
  }

  @override
  String deviceSystemFingerprint(String system) {
    return '$system · 指纹 ';
  }

  @override
  String get deviceControlsThis => '控制着本机';

  @override
  String devicePlaceAdjust(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {'left': '左侧', 'right': '右侧', 'top': '上方', 'other': '下方'});
    return '在本机$_temp0 · 调整位置';
  }

  @override
  String get deviceControlled => '本机现在是被控制的一方';

  @override
  String deviceControlledHint(String name) {
    return '鼠标、按键和防误切在本机控制它时才生效。被它控制时的滚动方向和按键，要在 $name 上改。';
  }

  @override
  String deviceOpenOn(String name) {
    return '在 $name 上打开';
  }

  @override
  String get devicePointerSpeed => '光标速度';

  @override
  String deviceBaseHere(int dpi) {
    return '$dpi · 本机';
  }

  @override
  String get deviceReset => '还原';

  @override
  String get deviceLocalPress => '本机按';

  @override
  String devicePeerGets(String name) {
    return '$name 收到';
  }

  @override
  String deviceModifierBecomes(String key, String name) {
    return '$key 在 $name 上变成';
  }

  @override
  String get deviceDeleteMapping => '删除这条映射';

  @override
  String get deviceLastAddress => '上次连上的地址';

  @override
  String get deviceLastAddressServerHint => '它上次从这个地址连过来';

  @override
  String get deviceLastAddressClientHint => '局域网里找不到它时，先试这个地址';

  @override
  String get deviceNotYet => '还没有';

  @override
  String get deviceInvite => '邀请它连过来';

  @override
  String get deviceInviteOnlineHint => '它已经连着本机';

  @override
  String get deviceInviteHint => '它在局域网里找不到本机、身边又没有键盘鼠标时用';

  @override
  String get deviceInviteButton => '邀请';

  @override
  String get deviceConnectMode => '连接方式';

  @override
  String get deviceFixedHint => '只连下面这个地址，不在局域网里找';

  @override
  String get deviceAutoHint => '在局域网里找控制端，找不到时用上次连上的地址';

  @override
  String get deviceAuto => '自动寻找';

  @override
  String get deviceFixed => '固定地址';

  @override
  String get deviceServerAddress => '控制端的地址';

  @override
  String get deviceAddressExample => '例如 192.168.2.12 或 192.168.2.12:24810';

  @override
  String get deviceRestoreDefaults => '恢复默认设置';

  @override
  String get deviceRestoreDefaultsHint => '鼠标、按键、切换与共享都回到默认，名字保留';

  @override
  String get deviceRestore => '恢复';

  @override
  String get deviceRemove => '移除这台设备';

  @override
  String get deviceRemoveConfirm => '确定吗？移除后需要重新配对，这台的设置一并删除';

  @override
  String get deviceRemoveHint => '移除后需要重新配对';

  @override
  String get deviceRemoveButton => '移除';

  @override
  String get phStarted => 'ScreenFerry 已启动';

  @override
  String phStartedServer(String name) {
    return '$name · 键鼠在本机';
  }

  @override
  String phStartedClient(String name) {
    return '$name · 由另一台电脑控制';
  }

  @override
  String get phCannotShare => '无法开始共享';

  @override
  String get phListening => '已开始共享';

  @override
  String phListeningAnnounced(String port) {
    return '等待另一台电脑连接 · 端口 $port';
  }

  @override
  String phListeningQuiet(String port) {
    return '端口 $port · 没有在局域网里广播';
  }

  @override
  String phConnected(String name) {
    return '已连接 $name';
  }

  @override
  String phDisconnected(String name) {
    return '与 $name 断开';
  }

  @override
  String get phPeerClosed => '对方已断开';

  @override
  String phPeerOpenedSettings(String name) {
    return '$name 请本机打开设置';
  }

  @override
  String get phShowingItsSettings => '已翻到它的设置页';

  @override
  String phCursorToPeer(String name) {
    return '光标切到 $name';
  }

  @override
  String phAcrossEdge(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {'left': '左侧', 'right': '右侧', 'top': '上方', 'other': '下方'});
    return '从本机$_temp0边缘';
  }

  @override
  String get phCursorBack => '光标回到本机';

  @override
  String phShortcut(String keys) {
    return '快捷键 $keys';
  }

  @override
  String get phFromPeerEdge => '从对方屏幕边缘';

  @override
  String get phCursorHere => '光标来到本机';

  @override
  String phFrom(String name) {
    return '从 $name';
  }

  @override
  String phCursorReturned(String name) {
    return '光标回到 $name';
  }

  @override
  String phClipToPeer(String name) {
    return '剪贴板同步到 $name';
  }

  @override
  String get phClipHere => '剪贴板同步到本机';

  @override
  String phClipTooBigToPeer(String format, String name) {
    String _temp0 = intl.Intl.selectLogic(format, {'image': '图片', 'text': '文本', 'other': '带格式的内容'});
    return '剪贴板里的$_temp0太大，没有同步到 $name';
  }

  @override
  String phClipTooBigHere(String format, String name) {
    String _temp0 = intl.Intl.selectLogic(format, {'image': '图片', 'text': '文本', 'other': '带格式的内容'});
    return '$name 剪贴板里的$_temp0太大，没有同步过来';
  }

  @override
  String phClipLimit(String size, String limit) {
    return '$size，超过了 $limit 的上限 · 可以在设置里调大';
  }

  @override
  String phPaired(String name) {
    return '已与 $name 配对';
  }

  @override
  String phRejectedByPeer(String name) {
    return '$name 拒绝了配对';
  }

  @override
  String phRejected(String name) {
    return '已拒绝 $name 的配对请求';
  }

  @override
  String phPairingRequest(String name) {
    return '$name 请求配对';
  }

  @override
  String phPairingCode(String code) {
    return '配对码 $code';
  }

  @override
  String phItems(String file, int items, int more) {
    return '$file 等 $items 项';
  }

  @override
  String phFiles(int files, String size) {
    return '$files 个文件 · $size';
  }

  @override
  String phSending(String name, String what) {
    return '发送 $what 到 $name';
  }

  @override
  String phReceiving(String name, String what) {
    return '$name 发来 $what';
  }

  @override
  String phSent(String what) {
    return '已发送 $what';
  }

  @override
  String phSentTo(String name, String size) {
    return '到 $name · $size';
  }

  @override
  String phReceived(String what) {
    return '已收到 $what';
  }

  @override
  String phSavedIn(String folder) {
    return '保存在 $folder';
  }

  @override
  String phInterrupted(String what) {
    return '传输中断：$what';
  }

  @override
  String get phResumesLater => '重新连上后接着传';

  @override
  String phCancelled(String what) {
    return '已取消：$what';
  }

  @override
  String get phCancelledByPeer => '对方取消了';

  @override
  String get phCancelledHere => '本机取消了';

  @override
  String phFailed(String what) {
    return '传输失败：$what';
  }

  @override
  String get phSharingResumed => '继续共享';

  @override
  String get phSharingResumedHint => '键鼠可以切到对方';

  @override
  String get phSharingPaused => '暂停共享';

  @override
  String get phSharingPausedHint => '键鼠留在本机';

  @override
  String get phArranged => '调整了设备的位置';

  @override
  String get phLayoutSaved => '布局已保存';

  @override
  String get phOtherDevices => '其他设备';

  @override
  String phMoved(String name, String where) {
    return '$name 移到$where';
  }

  @override
  String get phNowServer => '改为控制别的电脑';

  @override
  String get phNowServerHint => '键盘鼠标在本机，等另一台电脑来连接';

  @override
  String get phNowClient => '改为由另一台电脑控制';

  @override
  String get phNowClientHint => '在局域网里寻找键盘鼠标所在的那台';

  @override
  String phRemoved(String name) {
    return '已移除 $name';
  }

  @override
  String get phPairAgain => '下次连接需要重新配对';

  @override
  String toastPeerSettingsOpened(String name) {
    return '已在 $name 上打开设置，把光标移过去就能改';
  }

  @override
  String toastPeerSettingsUnsupported(String name) {
    return '$name 上的 ScreenFerry 版本较旧，请在那台电脑上打开设置';
  }

  @override
  String get toastDiagnosticsMissing => '诊断文件已经不在了，请重新导出';

  @override
  String get toastNotConnectedPeer => '还没有连上对方，或者对方不支持传文件';

  @override
  String toastFilesDisabled(String name) {
    return '已在设置里关掉了拖文件给 $name';
  }

  @override
  String toastSending(String name) {
    return '开始发给 $name';
  }

  @override
  String get toastSharingResumed => '已继续共享';

  @override
  String get toastSharingPaused => '已暂停共享';

  @override
  String toastAutostartFailed(String message) {
    return '无法修改开机自启：$message';
  }

  @override
  String toastSaveFailed(String message) {
    return '保存设置失败：$message';
  }

  @override
  String toastPeerOffline(String name) {
    return '$name 现在没连着';
  }

  @override
  String toastPeersReadFailed(String message) {
    return '读取已配对设备失败：$message';
  }

  @override
  String toastRemoveFailed(String message) {
    return '移除失败：$message';
  }

  @override
  String get toastServerGone => '这台电脑已经不在局域网里了';

  @override
  String get toastPausedConnectLater => '已暂停共享，继续共享后再连接';

  @override
  String get toastNotSharingInviteLater => '还没开始共享，开始后再邀请';

  @override
  String toastDiagnosticsFailed(String message) {
    return '导出诊断文件失败：$message';
  }

  @override
  String layoutDropAtNth(String side, int n) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': '左侧，从上往下',
      'right': '右侧，从上往下',
      'top': '上方，从左往右',
      'other': '下方，从左往右',
    });
    return '松手后放到本机$_temp0第 $n 台';
  }

  @override
  String aboutVersionLine(String version) {
    return '版本 $version · 在多台电脑之间共享一套键盘鼠标';
  }

  @override
  String updateCurrent(String version) {
    return '当前版本 $version';
  }

  @override
  String get updateUpToDate => '已是最新版本';

  @override
  String updateCheckedAgo(String version, String ago) {
    return '$version · $ago检查';
  }

  @override
  String get updateChecking => '正在检查更新…';

  @override
  String get updateCheckingHint => '只取一份很小的更新信息，不发送任何设备信息';

  @override
  String updateFound(String version) {
    return '发现新版本 $version';
  }

  @override
  String updateReleased(String date) {
    return '$date发布';
  }

  @override
  String get updateDownloadedHint => '已在后台下载好';

  @override
  String get updateDownload => '下载';

  @override
  String updateDownloading(String version) {
    return '正在下载 $version';
  }

  @override
  String get updateRestartHint => '重启时键鼠会断开约 5 秒';

  @override
  String get updateLater => '稍后';

  @override
  String get updateRestart => '重启并更新';

  @override
  String get updateInstall => '安装';

  @override
  String get updateLocalConfirm => '安装时 Windows 会弹出管理员确认框，从另一台电脑操作时点不到它。请用这台电脑自己的键盘鼠标点“安装”';

  @override
  String updateInstalling(String version) {
    return '正在安装 $version…';
  }

  @override
  String get updateInstallingHint => 'ScreenFerry 会自己重新打开，这个窗口随后重新连上';

  @override
  String updateManual(String version) {
    return '发现新版本 $version，要手动安装';
  }

  @override
  String get updateManualHint => '这份不能自己更新：可能是自己打包的开发版本，或者放在没有写权限的位置';

  @override
  String get updateOpenDownloads => '打开下载页面';

  @override
  String get updateUnreachable => '连不上更新服务器';

  @override
  String get updateUnreachableHint => '已按系统的代理设置连接，也可以在浏览器里下载';

  @override
  String get updateBrowserDownload => '浏览器下载';

  @override
  String get updateRetry => '重试';

  @override
  String get updateVerifyFailed => '安装包没有通过校验，已删除';

  @override
  String get updateVerifyFailedHint => '什么都没有安装。可能是网络里有东西改了文件，换个网络再试';

  @override
  String get updateStale => '无法确认是不是最新版';

  @override
  String get updateStaleHint => '拿到的更新信息已经过期，可能有人挡住了新版本';

  @override
  String get updateNoKeys => '这份程序不能检查更新';

  @override
  String get updateNoKeysHint => '它没有内置发布公钥，通常是自己打包的开发版本';

  @override
  String get updateInstallFailed => '更新没有装上';

  @override
  String get updateBreaksPeers => '两台要一起更新';

  @override
  String updateBreaksPeersHint(String version) {
    return '$version 改了两台之间的通信方式：另一台也更新到 $version 之后才能再连上';
  }

  @override
  String get updateAuto => '自动检查更新';

  @override
  String get updateAutoHint => '每天检查一次，有新版本时在后台下载好，由你决定何时重启';

  @override
  String updateLastChecked(String ago) {
    return '上次检查：$ago';
  }

  @override
  String updateSidebar(String version) {
    return '更新到 $version';
  }

  @override
  String updateBannerTitle(String version) {
    return '新版本 $version 已准备好';
  }

  @override
  String get updateBannerHint => '重启 ScreenFerry 完成更新，键鼠会断开约 5 秒';

  @override
  String devicePeerCanUpdate(String version) {
    return '可以更新到 $version';
  }

  @override
  String phUpdateInstalled(String to) {
    return '已更新到 $to';
  }

  @override
  String phFromVersion(String from) {
    return '从 $from';
  }

  @override
  String phUpdateNotInstalled(String version) {
    return '$version 没有装上';
  }

  @override
  String phStillOn(String current) {
    return '还在用 $current';
  }

  @override
  String get toastUpdateCancelled => '没有得到管理员确认，没有安装';

  @override
  String toastUpdateFailed(String message) {
    return '更新没有装上：$message';
  }
}
