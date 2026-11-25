// 界面状态。业务数据全部来自后台（经 Backend），命令也经 Backend 发出；
// 这里只多管导航、侧栏、提示等纯界面状态，以及“先显示、等后台确认”的乐观更新。

import 'dart:async';
import 'dart:io' show exit;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../ipc/messages.dart';
import '../l10n/l10n.dart';
import 'arrangement.dart';
import 'backend.dart';
import 'demo_backend.dart';
import 'dial_text.dart';
import 'links.dart';
import 'phrases.dart';

export '../ipc/messages.dart';
export 'arrangement.dart';
export 'backend.dart' show Backend, DaemonLink, IpcBackend;
export 'demo_backend.dart' show DemoBackend;

enum SfPage { overview, devices, layout, log, settings, about }

extension ScreenSideLabel on ScreenSide {
  String label(AppLocalizations l) => switch (this) {
    ScreenSide.left => l.sideLeft,
    ScreenSide.right => l.sideRight,
    ScreenSide.top => l.sideTop,
    ScreenSide.bottom => l.sideBottom,
  };
}

enum SwitchMode { instant, dwell }

extension LogKindLabel on LogKind {
  String label(AppLocalizations l) => switch (this) {
    LogKind.cursor => l.logCursor,
    LogKind.clip => l.logClipboard,
    LogKind.link => l.logLinks,
    LogKind.file => l.logFiles,
  };
}

extension LogIconData on LogIcon {
  IconData get data => switch (this) {
    LogIcon.power => Icons.power_settings_new_rounded,
    LogIcon.link => Icons.link_rounded,
    LogIcon.unlink => Icons.link_off_rounded,
    LogIcon.lock => Icons.lock_rounded,
    LogIcon.cursor => Icons.near_me_rounded,
    LogIcon.clipboard => Icons.content_paste_rounded,
    LogIcon.pause => Icons.pause_rounded,
    LogIcon.play => Icons.play_arrow_rounded,
    LogIcon.layout => Icons.splitscreen_rounded,
    LogIcon.radar => Icons.radar_rounded,
    LogIcon.warning => Icons.warning_amber_rounded,
    LogIcon.settings => Icons.tune_rounded,
    LogIcon.file => Icons.insert_drive_file_rounded,
  };
}

/// 后台报告的系统对应的图标；未知系统不冒充 Mac 或 Windows。
IconData platformIconOf(String platform) => switch (platform) {
  'macos' => Icons.desktop_mac_outlined,
  'windows' => Icons.desktop_windows_outlined,
  // Material 没有 Linux 专用桌面图标，用中性电脑图标。
  'linux' => Icons.computer_outlined,
  _ => Icons.devices_other_outlined,
};

/// 后台报告的系统显示给用户的名字。
String platformLabelOf(String platform) => switch (platform) {
  'macos' => 'macOS',
  'windows' => 'Windows',
  'linux' => 'Linux',
  _ => 'Unknown',
};

enum KeyboardKind { mac, xkb, unknown }

KeyboardKind keyboardKindOf(String platform) => switch (platform) {
  'macos' => KeyboardKind.mac,
  'windows' || 'linux' => KeyboardKind.xkb,
  _ => KeyboardKind.unknown,
};

/// 外观单独通知，避免每秒的数据刷新重建整个 MaterialApp。
class Appearance extends ChangeNotifier {
  ThemeMode _mode = ThemeMode.system;
  int _accent = 0;
  UiLanguage _language = UiLanguage.system;

  ThemeMode get mode => _mode;
  int get accent => _accent;
  UiLanguage get language => _language;

  void apply({ThemeMode? mode, int? accent, UiLanguage? language}) {
    final m = mode ?? _mode;
    final a = accent ?? _accent;
    final l = language ?? _language;
    if (m == _mode && a == _accent && l == _language) return;
    _mode = m;
    _accent = a;
    _language = l;
    notifyListeners();
  }

  /// 系统的语言变了：跟随系统时要换语言。
  void refresh() => notifyListeners();
}

class _LocaleWatch with WidgetsBindingObserver {
  _LocaleWatch(this.onChange);

  final VoidCallback onChange;

  @override
  void didChangeLocales(List<Locale>? locales) => onChange();
}

/// 底部提示。`id` 递增，同一句话连续出现也会重新弹出。
class ToastMessage {
  const ToastMessage(this.id, this.text);

  final int id;
  final String text;
}

/// 今天（本地时区）的活动汇总。
class TodayStats {
  const TodayStats({required this.switches, required this.clips, required this.connectedHours, required this.hours});

  final int switches;
  final int clips;
  final double connectedHours;

  /// 0–23 时每小时的切换次数。
  final List<int> hours;
}

/// 一次按地址连接或邀请。
class DialAttempt {
  const DialAttempt({required this.address, required this.invite, this.done = false, this.error});

  final String address;
  final bool invite;

  /// 有结果了。
  final bool done;

  /// 没成的原因；还在连、或者成了时为 null。
  final DialError? error;

  bool get succeeded => done && error == null;
}

/// 乐观更新等待后台确认的最长时间，超时就以后台为准。
const _pendingTimeout = Duration(seconds: 3);

/// 按地址连接、邀请最多等这么久；后台一直没给结果时按连不上处理。
const _dialTimeout = Duration(seconds: 20);

/// 看运行日志时多久读一次新写的。
const _runLogPoll = Duration(seconds: 2);

/// 界面上最多留这么多行运行日志。
const _runLogKeep = 1000;

/// 提问题的地方：GitHub 的问题表单，版本和系统预先填好。
const _issueUrl = 'https://github.com/icloudza/screen-ferry/issues/new';

/// 下载页面：不能自己更新时请用户来这里手动安装。
const _releasesUrl = 'https://github.com/icloudza/screen-ferry/releases/latest';

/// 已经导出的诊断文件。
class ExportedDiagnostics {
  const ExportedDiagnostics({required this.path, required this.bytes});
  final String path;
  final int bytes;

  /// 文件名（不含目录）。
  String get name => path.split(RegExp(r'[/\\]')).last;
}

class AppState extends ChangeNotifier {
  AppState({Backend? backend, LinkOpener? openLink, VoidCallback? onQuit})
    : backend = backend ?? DemoBackend(),
      _openLink = openLink ?? openInBrowser,
      _onQuit = onQuit ?? _exitApp {
    _link = this.backend.link.value;
    this.backend.link.addListener(_onLink);
    _subscription = this.backend.notices.listen(_onNotice);
    WidgetsBinding.instance.addObserver(_localeWatch);
  }

  /// 跟随系统时，系统换了语言就重画。
  late final _localeWatch = _LocaleWatch(appearance.refresh);

  final Backend backend;
  final LinkOpener _openLink;

  /// 后台要退出时界面怎么关闭。测试里换掉，不真的退出进程。
  final VoidCallback _onQuit;
  static void _exitApp() => exit(0);
  late final StreamSubscription<Notice> _subscription;

  final appearance = Appearance();
  final toast = ValueNotifier<ToastMessage?>(null);
  int _toastId = 0;

  // ── 导航 ──
  SfPage page = SfPage.overview;
  bool sidebarCollapsed = false;

  /// 设备页上打开着设置页的那台设备（指纹）；在设备列表时为 null。
  String? openedDevice;

  // ── 与后台的连接 ──
  late DaemonLink _link;
  DaemonState? _daemon;

  DaemonLink get daemonLink => _link;

  /// 最近一次收到的状态快照；还没连上过后台时为 null。
  DaemonState? get daemon => _daemon;

  /// 已连上后台并拿到了状态，可以发命令。
  bool get online => _link == DaemonLink.online && _daemon != null;

  /// 连上过后台、现在断开了。
  bool get disconnected => _daemon != null && _link != DaemonLink.online;

  // ── 乐观更新 ──
  SettingsPatch _pending = const SettingsPatch();
  DateTime _pendingAt = DateTime.fromMillisecondsSinceEpoch(0);

  /// 改了还没等到后台确认的设备设置，按指纹。
  final _pendingDevices = <String, (DeviceSettings, DateTime)>{};
  bool? _pendingSharing;
  DateTime _pendingSharingAt = DateTime.fromMillisecondsSinceEpoch(0);
  final _forgotten = <String>{};
  final _freshPeers = <String, DateTime>{};

  // ── 按地址连接与邀请 ──
  DialAttempt? _dial;
  Timer? _dialTimer;

  // ── 配对与日志 ──
  final _prompts = <PairingPrompt>[];
  final List<LogEntry> log = [];

  void start() => backend.start();

  // ── 本机 ──

  Role get role => _daemon?.local.role ?? Role.server;
  bool get isServer => role == Role.server;

  /// 本机系统：`macos`、`windows` 等。
  String get platform => _daemon?.local.platform ?? '';

  /// 本机键盘类型。未知系统不按连接角色或非 Mac 默认值猜测。
  KeyboardKind get localKeyboard => keyboardKindOf(platform);

  /// 本机是不是 Mac。
  bool get localIsMac => localKeyboard == KeyboardKind.mac;

  /// 本机和当前对方的图标。对方还没报系统时用中性图标，不按本机反推。
  IconData get localIcon => platformIconOf(platform);
  IconData get peerIcon {
    final current = peer;
    return platformIconOf(current == null ? '' : peerOf(current.fingerprint)?.platform ?? '');
  }

  String get localName => _daemon?.local.name ?? '';
  String get localFingerprint => _daemon?.local.short ?? '';
  String get dataDir => _daemon?.local.dataDir ?? '';

  /// 本机的局域网地址，常见网段在前。
  List<String> get localAddresses => _daemon?.local.addresses ?? const [];

  /// macOS：本机开着“自然滚动”。
  bool get naturalScrolling => _daemon?.local.naturalScrolling ?? false;
  String get version => _daemon?.local.version ?? '';

  // ── 版本更新 ──

  UpdateInfo get updateInfo => _daemon?.update ?? const UpdateInfo();

  /// 侧栏和托盘提醒的版本：下载好可以重启更新的，或者发现了但不能自己装的。
  String? get updateReminder {
    final u = updateInfo;
    final latest = u.latest;
    if (latest == null) return null;
    if (u.status == UpdateStatus.ready) return latest.version;
    if (u.status == UpdateStatus.available && !u.canInstall) return latest.version;
    return null;
  }

  /// 仪表盘横幅：下载好、用户还没点过“稍后”。
  bool get showUpdateBanner {
    final ready = updateInfo.readyVersion;
    return ready != null && ready != updateInfo.dismissed;
  }

  void checkUpdate() => _send(const CheckUpdateRequest());
  void downloadUpdate() => _send(const DownloadUpdateRequest());
  void cancelUpdate() => _send(const CancelUpdateRequest());
  void installUpdate() => _send(const InstallUpdateRequest());
  void dismissUpdate(String version) => _send(DismissUpdateRequest(version));

  /// 在浏览器里打开下载页面。
  Future<void> openReleases() => openLink(_releasesUrl);

  // ── 连接 ──

  bool get sharing => _pendingSharing ?? _daemon?.sharing ?? false;

  LinkStatus get status {
    if (_pendingSharing == false) return LinkStatus.paused;
    if (_pendingSharing == true && _daemon?.link.status == LinkStatus.paused) return LinkStatus.starting;
    return _daemon?.link.status ?? LinkStatus.starting;
  }

  bool get connected => status == LinkStatus.connected;
  PeerRef? get peer => connected ? _daemon?.link.peer : null;
  DateTime? get connectedAt => connected ? _daemon?.link.since : null;
  bool get cursorOnPeer => connected && (_daemon?.link.cursorOnPeer ?? false);
  Problem? get problem => _daemon?.problem;

  /// 对方的名字；没连上时取第一台已配对的设备，再没有就叫“对方”。
  /// 连着几台时 [peer] 是光标所在（否则最近连上）的那台。本机给它起过名字就用起的。
  String get peerName {
    final ref = peer;
    if (ref != null) return displayNameOf(ref.fingerprint, ref.name);
    return peers.firstOrNull?.displayName ?? l10n.peerFallback;
  }

  /// 一台设备显示给用户的名字；不是已配对的设备时用 [fallback]。
  String displayNameOf(String fingerprint, String fallback) => peerOf(fingerprint)?.displayName ?? fallback;

  /// 光标所在的那台设备；光标在本机或没在共享时为 null。
  PeerInfo? get cursorPeer => connected ? peers.where((p) => p.cursor).firstOrNull : null;

  /// 已连接的设备有几台。
  int get onlineCount => peers.where((p) => p.online).length;

  List<double> get rtt => connected ? (_daemon?.link.rttMs ?? const []) : const [];
  double? get latestRtt => rtt.isEmpty ? null : rtt.last;
  double? get peakRtt => rtt.isEmpty ? null : rtt.reduce((a, b) => a > b ? a : b);
  double? get avgRtt => rtt.isEmpty ? null : rtt.reduce((a, b) => a + b) / rtt.length;

  ClipboardSync? get clipboard => _daemon?.clipboard;

  // ── 设备 ──

  List<PeerInfo> get peers => [
    for (final p in _daemon?.peers ?? const <PeerInfo>[])
      if (!_forgotten.contains(p.fingerprint)) p,
  ];

  List<FoundDevice> get found => [
    for (final f in _daemon?.found ?? const <FoundDevice>[])
      if (!f.paired) f,
  ];

  /// 一台已配对的设备；不在列表里时为 null。
  PeerInfo? peerOf(String fingerprint) => peers.where((p) => p.fingerprint == fingerprint).firstOrNull;

  KeyboardKind peerKeyboard(PeerInfo p) => keyboardKindOf(p.platform);

  /// 只有后台明确报告 macOS 才按 Mac 处理。
  bool peerIsMac(PeerInfo p) => peerKeyboard(p) == KeyboardKind.mac;

  /// 对方的图标。
  IconData peerIconOf(PeerInfo p) => platformIconOf(p.platform);

  /// 两台是不是不同键盘；任一端系统未知时为 null，表示不能推断。
  bool? keyboardsDiffer(PeerInfo p) {
    final local = localKeyboard;
    final peer = peerKeyboard(p);
    if (local == KeyboardKind.unknown || peer == KeyboardKind.unknown) return null;
    return local != peer;
  }

  /// 刚配对上的设备，用来播放卡片进场动画。
  bool isFresh(String fingerprint) {
    final at = _freshPeers[fingerprint];
    return at != null && DateTime.now().difference(at) < const Duration(seconds: 2);
  }

  // ── 设置 ──

  DaemonSettings get settings {
    var s =
        (_daemon?.settings ??
                const DaemonSettings(
                  name: '',
                  side: ScreenSide.right,
                  announce: true,
                  clipboard: true,
                  server: '',
                  autostart: false,
                  theme: UiTheme.system,
                  accent: 0,
                ))
            .apply(_pending);
    for (final (device, _) in _pendingDevices.values) {
      s = s.withDevice(device);
    }
    return s;
  }

  /// 一台设备的单独设置（含还没确认的改动）；没有单独设置过的是缺省设置。
  DeviceSettings deviceSettings(String fingerprint) => settings.device(fingerprint);

  /// 没有单独安排过位置的设备放在本机哪一侧；只有一台设备时就是它所在的那一侧。
  ScreenSide get side => settings.side;

  /// 服务端：每台已配对设备在本机四周的位置。按设置（含还没确认的改动）算出。
  Arrangement get arrangement => Arrangement.of(peers, settings);

  /// 服务端：一台设备在本机哪一侧、排第几；不是已配对的设备时为 null。
  PeerPlace? placeOf(String fingerprint) => arrangement.placeOf(fingerprint);

  // ── 今日 ──

  TodayStats get today {
    final now = DateTime.now();
    final hours = List.filled(24, 0);
    var (switches, clips, secs) = (0, 0, 0);
    for (final a in _daemon?.activity ?? const <HourActivity>[]) {
      final t = a.hour.toLocal();
      if (t.year != now.year || t.month != now.month || t.day != now.day) continue;
      hours[t.hour] += a.switches;
      switches += a.switches;
      clips += a.clips;
      secs += a.connectedSecs;
    }
    return TodayStats(switches: switches, clips: clips, connectedHours: secs / 3600, hours: hours);
  }

  // ── 配对 ──

  /// 还在等待用户核对的配对请求，先来的在前。
  List<PairingPrompt> get prompts => List.unmodifiable(_prompts);

  bool isPromptOpen(int id) => _prompts.any((p) => p.id == id);

  // ── 界面状态 ──

  void update(VoidCallback change) {
    change();
    notifyListeners();
  }

  void go(SfPage target) {
    if (page == target && openedDevice == null) return;
    page = target;
    openedDevice = null;
    notifyListeners();
  }

  /// 打开一台设备的设置页。
  void openDevice(String fingerprint) {
    page = SfPage.devices;
    openedDevice = fingerprint;
    notifyListeners();
  }

  /// 从设备的设置页回到设备列表。
  void closeDevice() {
    if (openedDevice == null) return;
    openedDevice = null;
    notifyListeners();
  }

  void showToast(String text) => toast.value = ToastMessage(++_toastId, text);

  /// 用浏览器打开链接；打不开时复制下来并提示。
  Future<void> openLink(String url) async {
    if (await _openLink(url)) return;
    await Clipboard.setData(ClipboardData(text: url));
    showToast(l10n.toastLinkCopied);
  }

  // ── 命令 ──

  bool _send(Request request) {
    if (online && backend.send(request)) return true;
    showToast(l10n.toastNotConnected);
    return false;
  }

  void setSharing(bool on) {
    if (!_send(SetSharingRequest(on))) return;
    _pendingSharing = on;
    _pendingSharingAt = DateTime.now();
    notifyListeners();
  }

  /// 修改设置：先按新值显示，等后台的状态确认。
  void updateSettings(SettingsPatch patch) {
    if (!_send(UpdateSettingsRequest(patch))) return;
    _pending = _pending.merge(patch);
    _pendingAt = DateTime.now();
    _applyAppearance();
    notifyListeners();
  }

  void setTheme(ThemeMode mode) => updateSettings(SettingsPatch(theme: UiTheme.values.byName(mode.name)));

  void setLanguage(UiLanguage language) => updateSettings(SettingsPatch(language: language));

  /// 实际用的语言：设置里选的，跟随系统时看系统的首选语言。
  Locale get locale => resolveLocale(appearance.language, WidgetsBinding.instance.platformDispatcher.locale);

  /// 当前语言的文字，给没有 BuildContext 的地方用（提示、日志）。
  AppLocalizations get l10n => lookupAppLocalizations(locale);

  void setAccent(int accent) => updateSettings(SettingsPatch(accent: accent));

  /// 改为控制别的电脑（服务端）或被控制（客户端）。
  void setRole(Role to) {
    if (to != role) updateSettings(SettingsPatch(role: to));
  }

  /// 改变没有单独安排的设备放在哪一侧。
  void moveScreen(ScreenSide to) {
    if (to != side) updateSettings(SettingsPatch(side: to));
  }

  /// 把一台设备移到本机 [to] 一侧的第 [index] 位（按去掉它之后的那一侧算）。
  /// 发出的 layout 列出全部已配对设备，之后每台都是单独安排过的。位置没变时不发。
  void moveDevice(String fingerprint, ScreenSide to, int index) {
    final now = arrangement;
    final next = now.moved(fingerprint, to, index);
    if (next.sameAs(now)) return;
    updateSettings(SettingsPatch(layout: next.toLayout()));
  }

  void forget(PeerInfo peer) {
    if (!_send(ForgetRequest(peer.fingerprint))) return;
    _forgotten.add(peer.fingerprint);
    if (openedDevice == peer.fingerprint) openedDevice = null;
    notifyListeners();
  }

  /// 换上一台设备的设置：先按新值显示，等后台的状态确认。
  void setDevice(DeviceSettings device) {
    if (!_send(SetDeviceRequest(device))) return;
    _pendingDevices[device.fingerprint] = (device, DateTime.now());
    notifyListeners();
  }

  /// 恢复默认设置，保留名字。
  void resetDevice(String fingerprint) => setDevice(deviceSettings(fingerprint).reset());

  /// 本机被控制时：请对方打开它的设置界面，翻到本机的设置页。
  void openPeerSettings(PeerInfo peer) => _send(OpenPeerSettingsRequest(peer.fingerprint));

  /// 正在进行或刚有结果的按地址连接、邀请；没有时为 null。
  DialAttempt? get dial => _dial;

  /// 按地址连接（客户端）或邀请（服务端）。结果见 [dial]。
  void dialAddress(String address, {required bool invite}) {
    final text = address.trim();
    if (text.isEmpty) return;
    if (!_send(invite ? InviteRequest(text) : ConnectAddressRequest(text))) return;
    _dial = DialAttempt(address: text, invite: invite);
    _dialTimer?.cancel();
    _dialTimer = Timer(_dialTimeout, () {
      final d = _dial;
      if (d == null || d.done) return;
      _dial = DialAttempt(address: d.address, invite: d.invite, done: true, error: DialError.unreachable);
      notifyListeners();
    });
    notifyListeners();
  }

  /// 关掉按地址连接、邀请的对话框时调用。
  void clearDial() {
    _dialTimer?.cancel();
    if (_dial == null) return;
    _dial = null;
    notifyListeners();
  }

  void connectTo(FoundDevice device) {
    if (_send(ConnectRequest(device.fingerprint))) showToast(l10n.toastConnectingTo(device.name));
  }

  void replyPairing(int id, {required bool accept}) => _send(PairReplyRequest(id, accept: accept));

  // ── 运行日志与诊断文件 ──

  /// 运行日志，旧的在前，最多 [_runLogKeep] 行。
  final List<RunLogLine> runLog = [];

  /// 当前这卷运行日志的大小和行数；还没读过时为 null。
  int? runLogBytes;
  int? runLogLines;
  int? _runLogNext;
  Timer? _runLogTimer;

  /// 最近导出的诊断文件。
  ExportedDiagnostics? lastExport;

  /// 详细记录从什么时候开着；关着时为 null。
  DateTime? get verboseSince => _daemon?.local.verboseSince;

  /// 本机系统和版本，例如“macOS 15.6 · arm64”。
  String get system => _daemon?.local.system ?? '';

  /// 开始或停止跟着看运行日志：打开时先读最近的一段，之后每隔一会儿读新写的。
  void watchRunLog(bool on) {
    _runLogTimer?.cancel();
    _runLogTimer = null;
    if (!on) return;
    _runLogNext = null;
    _readRunLog();
    _runLogTimer = Timer.periodic(_runLogPoll, (_) => _readRunLog());
  }

  void _readRunLog() => _send(ReadRunLogRequest(since: _runLogNext));

  void exportDiagnostics() => _send(const ExportDiagnosticsRequest());

  void copyDiagnostics() => _send(const CopyDiagnosticsRequest());

  void revealDiagnostics() => _send(const RevealDiagnosticsRequest());

  void openLogsFolder() => _send(const OpenLogsFolderRequest());

  void setVerbose(bool on) => _send(SetVerboseRequest(on));

  /// 在浏览器里打开 GitHub 的问题表单，版本和系统预先填好。
  Future<void> openIssue() => openLink(
    Uri.parse(_issueUrl)
        .replace(
          queryParameters: {
            'template': 'bug_report.yml',
            if (version.isNotEmpty) 'version': version,
            if (system.isNotEmpty) 'system': system,
          },
        )
        .toString(),
  );

  void clearLog() {
    if (!_send(const ClearLogRequest())) return;
    log.clear();
    notifyListeners();
  }

  void openPermissionSettings() => _send(const OpenPermissionSettingsRequest());

  // ── 后台推来的 ──

  void _onLink() {
    _link = backend.link.value;
    // 后台换了一份（重启过）：运行日志从头读。
    _runLogNext = null;
    notifyListeners();
  }

  void _onNotice(Notice notice) {
    switch (notice) {
      case StateNotice(:final state):
        _onState(state);
      case HistoryNotice(:final entries):
        log
          ..clear()
          ..addAll(entries);
      case LogNotice(:final entry):
        log.insert(0, entry);
        if (log.length > 500) log.removeLast();
      case PairingNotice(:final prompt):
        _prompts
          ..removeWhere((p) => p.id == prompt.id)
          ..add(prompt);
      case PairingClosedNotice(:final id):
        _prompts.removeWhere((p) => p.id == id);
      case ToastNotice(:final text, :final phrase):
        showToast((phrase == null ? null : toastText(l10n, phrase)) ?? text);
      case ShowDeviceNotice(:final fingerprint):
        openDevice(fingerprint);
        return;
      case DialedNotice(:final address, :final invite, :final error):
        final d = _dial;
        if (d != null && !d.done && d.invite == invite) {
          _dialTimer?.cancel();
          _dial = DialAttempt(address: d.address, invite: invite, done: true, error: error);
        } else if (error != null) {
          // 没有对话框在等（例如点了局域网里发现的设备）：失败时提示一下。
          showToast(dialFailure(l10n, error, address: address, invite: invite).title);
        }
      case QuitNotice():
        // 后台退出了（托盘里的“退出”或安装程序升级、卸载），设置界面跟着关闭。
        _onQuit();
        return;
      case RunLogNotice(:final lines, :final next, :final fresh, :final bytes, :final linesTotal):
        if (fresh) runLog.clear();
        runLog.addAll(lines);
        if (runLog.length > _runLogKeep) runLog.removeRange(0, runLog.length - _runLogKeep);
        _runLogNext = next;
        runLogBytes = bytes;
        runLogLines = linesTotal;
        // 没有新内容时不必重画。
        if (!fresh && lines.isEmpty) return;
      case DiagnosticsExportedNotice(:final path, :final bytes):
        lastExport = ExportedDiagnostics(path: path, bytes: bytes);
      case DiagnosticsSummaryNotice(:final text):
        unawaited(Clipboard.setData(ClipboardData(text: text)));
        showToast(l10n.diagCopied);
        return;
      case WelcomeNotice():
        return;
    }
    notifyListeners();
  }

  void _onState(DaemonState state) {
    final before = _daemon?.peers.map((p) => p.fingerprint).toSet();
    final now = DateTime.now();
    if (before != null) {
      for (final p in state.peers) {
        if (!before.contains(p.fingerprint)) _freshPeers[p.fingerprint] = now;
      }
    }
    _daemon = state;

    // 后台已经确认的改动不再覆盖；等太久的以后台为准。
    _pending = now.difference(_pendingAt) > _pendingTimeout ? const SettingsPatch() : _pending.without(state.settings);
    if (_pendingSharing == state.sharing || now.difference(_pendingSharingAt) > _pendingTimeout) {
      _pendingSharing = null;
    }
    _pendingDevices.removeWhere(
      (fp, pending) => now.difference(pending.$2) > _pendingTimeout || state.settings.device(fp) == pending.$1,
    );
    _forgotten.retainWhere((fp) => state.peers.any((p) => p.fingerprint == fp));
    _applyAppearance();
  }

  void _applyAppearance() {
    final s = settings;
    appearance.apply(mode: ThemeMode.values.byName(s.theme.name), accent: s.accent, language: s.language);
  }

  @override
  void dispose() {
    _dialTimer?.cancel();
    _runLogTimer?.cancel();
    WidgetsBinding.instance.removeObserver(_localeWatch);
    unawaited(_subscription.cancel());
    backend.link.removeListener(_onLink);
    unawaited(backend.dispose());
    appearance.dispose();
    toast.dispose();
    super.dispose();
  }
}

/// “12 秒前”这样的相对时间。
String timeAgo(AppLocalizations l, DateTime at) {
  final s = DateTime.now().difference(at).inSeconds;
  if (s < 5) return l.timeJustNow;
  if (s < 60) return l.timeSecondsAgo(s);
  if (s < 3600) return l.timeMinutesAgo(s ~/ 60);
  if (s < 86400) return l.timeHoursAgo(s ~/ 3600);
  return l.timeDaysAgo(s ~/ 86400);
}

/// 已配对设备的在线情况：“在线”“5 分钟前连接过”“离线”。
String seenText(AppLocalizations l, PeerInfo p) {
  if (p.online) return l.seenOnline;
  final at = p.lastSeen;
  return at == null ? l.seenOffline : l.seenAgo(timeAgo(l, at));
}

/// “14:32:05”这样的时刻。
String clockTime(DateTime at) {
  String two(int v) => v.toString().padLeft(2, '0');
  return '${two(at.hour)}:${two(at.minute)}:${two(at.second)}';
}

/// “1 小时 12 分”这样的时长。
String durationText(AppLocalizations l, Duration d) {
  final m = d.inMinutes;
  if (m < 1) return l.durationUnderMinute;
  if (m < 60) return l.durationMinutes(m);
  return l.durationHours(m ~/ 60, m % 60);
}

/// 字节数写成“93 B”“1.2 MB”。
String sizeText(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  return '${(bytes / 1024 / 1024).toStringAsFixed(1)} MB';
}
