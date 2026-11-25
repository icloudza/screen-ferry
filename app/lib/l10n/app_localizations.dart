import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale) : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate = _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates = <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[Locale('en'), Locale('zh')];

  /// No description provided for @appTitle.
  ///
  /// In zh, this message translates to:
  /// **'ScreenFerry'**
  String get appTitle;

  /// No description provided for @commonSave.
  ///
  /// In zh, this message translates to:
  /// **'保存'**
  String get commonSave;

  /// No description provided for @commonCancel.
  ///
  /// In zh, this message translates to:
  /// **'取消'**
  String get commonCancel;

  /// No description provided for @commonClose.
  ///
  /// In zh, this message translates to:
  /// **'关闭'**
  String get commonClose;

  /// No description provided for @navOverview.
  ///
  /// In zh, this message translates to:
  /// **'仪表盘'**
  String get navOverview;

  /// No description provided for @navDevices.
  ///
  /// In zh, this message translates to:
  /// **'设备'**
  String get navDevices;

  /// No description provided for @navLayout.
  ///
  /// In zh, this message translates to:
  /// **'布局'**
  String get navLayout;

  /// No description provided for @navLog.
  ///
  /// In zh, this message translates to:
  /// **'日志'**
  String get navLog;

  /// No description provided for @navSettings.
  ///
  /// In zh, this message translates to:
  /// **'设置'**
  String get navSettings;

  /// No description provided for @sidebarExpand.
  ///
  /// In zh, this message translates to:
  /// **'展开侧栏'**
  String get sidebarExpand;

  /// No description provided for @sidebarCollapse.
  ///
  /// In zh, this message translates to:
  /// **'收起侧栏'**
  String get sidebarCollapse;

  /// No description provided for @statusConnectingDaemon.
  ///
  /// In zh, this message translates to:
  /// **'正在连接后台'**
  String get statusConnectingDaemon;

  /// No description provided for @statusDaemonNotRunning.
  ///
  /// In zh, this message translates to:
  /// **'后台未运行'**
  String get statusDaemonNotRunning;

  /// No description provided for @statusDaemonLost.
  ///
  /// In zh, this message translates to:
  /// **'与后台断开'**
  String get statusDaemonLost;

  /// No description provided for @statusSharing.
  ///
  /// In zh, this message translates to:
  /// **'共享中'**
  String get statusSharing;

  /// No description provided for @statusSharingRtt.
  ///
  /// In zh, this message translates to:
  /// **'共享中 · {ms} ms'**
  String statusSharingRtt(String ms);

  /// No description provided for @statusPaused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停共享'**
  String get statusPaused;

  /// No description provided for @statusWaiting.
  ///
  /// In zh, this message translates to:
  /// **'等待对方连接'**
  String get statusWaiting;

  /// No description provided for @statusSearching.
  ///
  /// In zh, this message translates to:
  /// **'正在寻找对方'**
  String get statusSearching;

  /// No description provided for @statusConnecting.
  ///
  /// In zh, this message translates to:
  /// **'正在连接'**
  String get statusConnecting;

  /// No description provided for @statusStarting.
  ///
  /// In zh, this message translates to:
  /// **'正在启动'**
  String get statusStarting;

  /// No description provided for @statusStopped.
  ///
  /// In zh, this message translates to:
  /// **'已停止'**
  String get statusStopped;

  /// No description provided for @windowMinimize.
  ///
  /// In zh, this message translates to:
  /// **'最小化'**
  String get windowMinimize;

  /// No description provided for @windowRestore.
  ///
  /// In zh, this message translates to:
  /// **'还原'**
  String get windowRestore;

  /// No description provided for @windowMaximize.
  ///
  /// In zh, this message translates to:
  /// **'最大化'**
  String get windowMaximize;

  /// No description provided for @chartSwitchesLabel.
  ///
  /// In zh, this message translates to:
  /// **'今日各小时的切换次数'**
  String get chartSwitchesLabel;

  /// No description provided for @chartMidnight.
  ///
  /// In zh, this message translates to:
  /// **'0 时'**
  String get chartMidnight;

  /// No description provided for @chartNoon.
  ///
  /// In zh, this message translates to:
  /// **'12 时'**
  String get chartNoon;

  /// No description provided for @chartNow.
  ///
  /// In zh, this message translates to:
  /// **'现在'**
  String get chartNow;

  /// No description provided for @bannerDaemonLost.
  ///
  /// In zh, this message translates to:
  /// **'与后台的连接已断开，正在重连…'**
  String get bannerDaemonLost;

  /// No description provided for @bannerPermission.
  ///
  /// In zh, this message translates to:
  /// **'需要键鼠控制权限'**
  String get bannerPermission;

  /// No description provided for @bannerPortInUse.
  ///
  /// In zh, this message translates to:
  /// **'端口被占用'**
  String get bannerPortInUse;

  /// No description provided for @bannerOther.
  ///
  /// In zh, this message translates to:
  /// **'后台遇到了问题'**
  String get bannerOther;

  /// No description provided for @bannerOpenSystemSettings.
  ///
  /// In zh, this message translates to:
  /// **'打开系统设置'**
  String get bannerOpenSystemSettings;

  /// No description provided for @problemPermission.
  ///
  /// In zh, this message translates to:
  /// **'需要在系统设置的“隐私与安全性”中允许 ScreenFerry 控制键盘和鼠标'**
  String get problemPermission;

  /// No description provided for @problemPortInUse.
  ///
  /// In zh, this message translates to:
  /// **'端口 {port} 被占用：可能还开着另一个键鼠共享软件或另一份 ScreenFerry'**
  String problemPortInUse(int port);

  /// No description provided for @offlineConnecting.
  ///
  /// In zh, this message translates to:
  /// **'正在连接后台…'**
  String get offlineConnecting;

  /// No description provided for @offlineNotRunning.
  ///
  /// In zh, this message translates to:
  /// **'后台没有运行'**
  String get offlineNotRunning;

  /// No description provided for @offlineStartHint.
  ///
  /// In zh, this message translates to:
  /// **'请先启动 ScreenFerry，连上后会自动显示。'**
  String get offlineStartHint;

  /// No description provided for @offlineAutoConnect.
  ///
  /// In zh, this message translates to:
  /// **'启动后会自动连上。'**
  String get offlineAutoConnect;

  /// No description provided for @offlineStartFailed.
  ///
  /// In zh, this message translates to:
  /// **'启动失败，请在“应用程序”中打开 ScreenFerry'**
  String get offlineStartFailed;

  /// No description provided for @offlineStarting.
  ///
  /// In zh, this message translates to:
  /// **'正在启动…'**
  String get offlineStarting;

  /// No description provided for @offlineStart.
  ///
  /// In zh, this message translates to:
  /// **'启动 ScreenFerry'**
  String get offlineStart;

  /// No description provided for @offlineRetrying.
  ///
  /// In zh, this message translates to:
  /// **'正在重试'**
  String get offlineRetrying;

  /// No description provided for @nameEditorRename.
  ///
  /// In zh, this message translates to:
  /// **'改名字'**
  String get nameEditorRename;

  /// No description provided for @accentBlue.
  ///
  /// In zh, this message translates to:
  /// **'蓝色'**
  String get accentBlue;

  /// No description provided for @accentTeal.
  ///
  /// In zh, this message translates to:
  /// **'青色'**
  String get accentTeal;

  /// No description provided for @accentPurple.
  ///
  /// In zh, this message translates to:
  /// **'紫色'**
  String get accentPurple;

  /// No description provided for @accentOrange.
  ///
  /// In zh, this message translates to:
  /// **'橙色'**
  String get accentOrange;

  /// No description provided for @accentPink.
  ///
  /// In zh, this message translates to:
  /// **'粉色'**
  String get accentPink;

  /// No description provided for @aboutTitle.
  ///
  /// In zh, this message translates to:
  /// **'关于'**
  String get aboutTitle;

  /// No description provided for @aboutBack.
  ///
  /// In zh, this message translates to:
  /// **'返回设置'**
  String get aboutBack;

  /// No description provided for @aboutTagline.
  ///
  /// In zh, this message translates to:
  /// **'在多台电脑之间共享一套键盘鼠标'**
  String get aboutTagline;

  /// No description provided for @aboutCheckUpdates.
  ///
  /// In zh, this message translates to:
  /// **'检查更新'**
  String get aboutCheckUpdates;

  /// No description provided for @aboutUpdatesSoon.
  ///
  /// In zh, this message translates to:
  /// **'自动检查更新即将支持'**
  String get aboutUpdatesSoon;

  /// No description provided for @aboutAuthor.
  ///
  /// In zh, this message translates to:
  /// **'作者主页'**
  String get aboutAuthor;

  /// No description provided for @aboutLicense.
  ///
  /// In zh, this message translates to:
  /// **'许可证'**
  String get aboutLicense;

  /// No description provided for @logTitle.
  ///
  /// In zh, this message translates to:
  /// **'日志'**
  String get logTitle;

  /// No description provided for @logClear.
  ///
  /// In zh, this message translates to:
  /// **'清空'**
  String get logClear;

  /// No description provided for @logFilter.
  ///
  /// In zh, this message translates to:
  /// **'筛选'**
  String get logFilter;

  /// No description provided for @logAll.
  ///
  /// In zh, this message translates to:
  /// **'全部'**
  String get logAll;

  /// No description provided for @logCursor.
  ///
  /// In zh, this message translates to:
  /// **'光标'**
  String get logCursor;

  /// No description provided for @logClipboard.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板'**
  String get logClipboard;

  /// No description provided for @logFiles.
  ///
  /// In zh, this message translates to:
  /// **'文件'**
  String get logFiles;

  /// No description provided for @logLinks.
  ///
  /// In zh, this message translates to:
  /// **'设备与连接'**
  String get logLinks;

  /// No description provided for @logRun.
  ///
  /// In zh, this message translates to:
  /// **'运行日志'**
  String get logRun;

  /// No description provided for @logEmpty.
  ///
  /// In zh, this message translates to:
  /// **'暂无记录'**
  String get logEmpty;

  /// No description provided for @logCount.
  ///
  /// In zh, this message translates to:
  /// **'共 {count} 条'**
  String logCount(int count);

  /// No description provided for @logRunSize.
  ///
  /// In zh, this message translates to:
  /// **'{size} · {lines} 行'**
  String logRunSize(String size, String lines);

  /// No description provided for @pairingTitle.
  ///
  /// In zh, this message translates to:
  /// **'配对'**
  String get pairingTitle;

  /// No description provided for @pairingWith.
  ///
  /// In zh, this message translates to:
  /// **'与“{name}”配对'**
  String pairingWith(String name);

  /// No description provided for @pairingCheck.
  ///
  /// In zh, this message translates to:
  /// **'请确认两台电脑上显示的是同一个配对码，一致才确认。'**
  String get pairingCheck;

  /// No description provided for @pairingPeerFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'对方指纹 '**
  String get pairingPeerFingerprint;

  /// No description provided for @pairingWaiting.
  ///
  /// In zh, this message translates to:
  /// **'已确认，等待对方确认…'**
  String get pairingWaiting;

  /// No description provided for @pairingAutoReject.
  ///
  /// In zh, this message translates to:
  /// **'{seconds} 秒后自动拒绝'**
  String pairingAutoReject(int seconds);

  /// No description provided for @pairingTimedOut.
  ///
  /// In zh, this message translates to:
  /// **'已超时'**
  String get pairingTimedOut;

  /// No description provided for @pairingReject.
  ///
  /// In zh, this message translates to:
  /// **'拒绝'**
  String get pairingReject;

  /// No description provided for @pairingAccept.
  ///
  /// In zh, this message translates to:
  /// **'数字一致，配对'**
  String get pairingAccept;

  /// No description provided for @pairingCodeLabel.
  ///
  /// In zh, this message translates to:
  /// **'配对码 {code}'**
  String pairingCodeLabel(String code);

  /// No description provided for @pairingDone.
  ///
  /// In zh, this message translates to:
  /// **'已与“{name}”配对'**
  String pairingDone(String name);

  /// No description provided for @pairingNoMoreAsking.
  ///
  /// In zh, this message translates to:
  /// **'以后连接不再询问'**
  String get pairingNoMoreAsking;

  /// No description provided for @dialBadAddressTitle.
  ///
  /// In zh, this message translates to:
  /// **'地址不对：{address}'**
  String dialBadAddressTitle(String address);

  /// No description provided for @dialBadAddressBody.
  ///
  /// In zh, this message translates to:
  /// **'填 IP（例如 192.168.2.12）、IP:端口，或者电脑名。'**
  String get dialBadAddressBody;

  /// No description provided for @dialNotWaitingInviteTitle.
  ///
  /// In zh, this message translates to:
  /// **'{address} 没有在等待邀请'**
  String dialNotWaitingInviteTitle(String address);

  /// No description provided for @dialNotWaitingInviteBody.
  ///
  /// In zh, this message translates to:
  /// **'确认那台电脑开着 ScreenFerry，并且选的是“被控制”。它的版本太旧时也不能被邀请，需要先升级。'**
  String get dialNotWaitingInviteBody;

  /// No description provided for @dialNotWaitingTitle.
  ///
  /// In zh, this message translates to:
  /// **'{address} 没有在等待连接'**
  String dialNotWaitingTitle(String address);

  /// No description provided for @dialNotWaitingBody.
  ///
  /// In zh, this message translates to:
  /// **'确认那台电脑开着 ScreenFerry，并且在设置里选的是“控制别的电脑”。'**
  String get dialNotWaitingBody;

  /// No description provided for @dialUnreachableTitle.
  ///
  /// In zh, this message translates to:
  /// **'连不上 {address}'**
  String dialUnreachableTitle(String address);

  /// No description provided for @dialUnreachableBody.
  ///
  /// In zh, this message translates to:
  /// **'确认两台在同一个网络里、地址没有填错。Windows 第一次运行 ScreenFerry 时会询问防火墙，请允许它通过。'**
  String get dialUnreachableBody;

  /// No description provided for @dialNotScreenFerryTitle.
  ///
  /// In zh, this message translates to:
  /// **'{address} 上不是 ScreenFerry'**
  String dialNotScreenFerryTitle(String address);

  /// No description provided for @dialNotScreenFerryBody.
  ///
  /// In zh, this message translates to:
  /// **'确认地址和端口没有填错。'**
  String get dialNotScreenFerryBody;

  /// No description provided for @dialIncompatibleTitle.
  ///
  /// In zh, this message translates to:
  /// **'版本不兼容'**
  String get dialIncompatibleTitle;

  /// No description provided for @dialIncompatibleBody.
  ///
  /// In zh, this message translates to:
  /// **'两台电脑上的 ScreenFerry 版本相差太多，请都升级到最新版。'**
  String get dialIncompatibleBody;

  /// No description provided for @dialBusyTitle.
  ///
  /// In zh, this message translates to:
  /// **'{address} 正连着另一台电脑'**
  String dialBusyTitle(String address);

  /// No description provided for @dialBusyBody.
  ///
  /// In zh, this message translates to:
  /// **'先在那台电脑上断开，再邀请。'**
  String get dialBusyBody;

  /// No description provided for @dialWrongRoleInviteTitle.
  ///
  /// In zh, this message translates to:
  /// **'本机现在是被控制的一方'**
  String get dialWrongRoleInviteTitle;

  /// No description provided for @dialWrongRoleInviteBody.
  ///
  /// In zh, this message translates to:
  /// **'邀请是控制端用的；被控制的一方请用“按地址连接”。'**
  String get dialWrongRoleInviteBody;

  /// No description provided for @dialWrongRoleTitle.
  ///
  /// In zh, this message translates to:
  /// **'本机现在是控制端'**
  String get dialWrongRoleTitle;

  /// No description provided for @dialWrongRoleBody.
  ///
  /// In zh, this message translates to:
  /// **'按地址连接是被控制的一方用的；控制端请用“邀请一台电脑”。'**
  String get dialWrongRoleBody;

  /// No description provided for @dialInviteTitle.
  ///
  /// In zh, this message translates to:
  /// **'邀请一台电脑'**
  String get dialInviteTitle;

  /// No description provided for @dialConnectTitle.
  ///
  /// In zh, this message translates to:
  /// **'按地址连接'**
  String get dialConnectTitle;

  /// No description provided for @dialInvited.
  ///
  /// In zh, this message translates to:
  /// **'已邀请 {address}，它正在连过来'**
  String dialInvited(String address);

  /// No description provided for @dialConnected.
  ///
  /// In zh, this message translates to:
  /// **'已连上 {address}'**
  String dialConnected(String address);

  /// No description provided for @dialInviteHint.
  ///
  /// In zh, this message translates to:
  /// **'那台电脑要开着 ScreenFerry，选的是“被控制”。地址在它的「设备」页上；不写端口时用 24811。'**
  String get dialInviteHint;

  /// No description provided for @dialConnectHint.
  ///
  /// In zh, this message translates to:
  /// **'可以是 IP、IP:端口或电脑名，不写端口时用 24810。地址在那台电脑的「设备」页上。'**
  String get dialConnectHint;

  /// No description provided for @dialInviteSubtitle.
  ///
  /// In zh, this message translates to:
  /// **'输入被控制那台的地址，它会自动连过来'**
  String get dialInviteSubtitle;

  /// No description provided for @dialConnectSubtitle.
  ///
  /// In zh, this message translates to:
  /// **'输入控制端（键盘鼠标所在那台）的地址'**
  String get dialConnectSubtitle;

  /// No description provided for @dialAddress.
  ///
  /// In zh, this message translates to:
  /// **'地址'**
  String get dialAddress;

  /// No description provided for @dialExample.
  ///
  /// In zh, this message translates to:
  /// **'例如 {address}'**
  String dialExample(String address);

  /// No description provided for @dialInviting.
  ///
  /// In zh, this message translates to:
  /// **'正在邀请 {address}…'**
  String dialInviting(String address);

  /// No description provided for @dialConnecting.
  ///
  /// In zh, this message translates to:
  /// **'正在连接 {address}…'**
  String dialConnecting(String address);

  /// No description provided for @dialRetry.
  ///
  /// In zh, this message translates to:
  /// **'重试'**
  String get dialRetry;

  /// No description provided for @dialInvite.
  ///
  /// In zh, this message translates to:
  /// **'邀请'**
  String get dialInvite;

  /// No description provided for @dialConnect.
  ///
  /// In zh, this message translates to:
  /// **'连接'**
  String get dialConnect;

  /// No description provided for @diagTitle.
  ///
  /// In zh, this message translates to:
  /// **'提问题时附上诊断文件'**
  String get diagTitle;

  /// No description provided for @diagBody.
  ///
  /// In zh, this message translates to:
  /// **'包含版本、系统、设置和最近的运行日志。设备名、用户名和文件名会换成代号，不含剪贴板内容和密钥。'**
  String get diagBody;

  /// No description provided for @diagCopySummary.
  ///
  /// In zh, this message translates to:
  /// **'复制摘要'**
  String get diagCopySummary;

  /// No description provided for @diagExport.
  ///
  /// In zh, this message translates to:
  /// **'导出诊断文件'**
  String get diagExport;

  /// No description provided for @diagOpenFolder.
  ///
  /// In zh, this message translates to:
  /// **'打开日志文件夹'**
  String get diagOpenFolder;

  /// No description provided for @diagReport.
  ///
  /// In zh, this message translates to:
  /// **'去 GitHub 提问题'**
  String get diagReport;

  /// No description provided for @diagSaved.
  ///
  /// In zh, this message translates to:
  /// **'已存到“下载”：'**
  String get diagSaved;

  /// No description provided for @diagShowInFinder.
  ///
  /// In zh, this message translates to:
  /// **'在访达中显示'**
  String get diagShowInFinder;

  /// No description provided for @diagShowInExplorer.
  ///
  /// In zh, this message translates to:
  /// **'在资源管理器中显示'**
  String get diagShowInExplorer;

  /// No description provided for @diagVerbose.
  ///
  /// In zh, this message translates to:
  /// **'详细记录'**
  String get diagVerbose;

  /// No description provided for @diagVerboseOff.
  ///
  /// In zh, this message translates to:
  /// **'排查连接问题时打开，会多记一些细节。重启 ScreenFerry 后自动关闭'**
  String get diagVerboseOff;

  /// No description provided for @diagVerboseJustOn.
  ///
  /// In zh, this message translates to:
  /// **'刚刚打开。问题出现后导出，再把它关掉'**
  String get diagVerboseJustOn;

  /// No description provided for @diagVerboseOnFor.
  ///
  /// In zh, this message translates to:
  /// **'已打开 {minutes} 分钟。问题出现后导出，再把它关掉'**
  String diagVerboseOnFor(int minutes);

  /// No description provided for @diagRecent.
  ///
  /// In zh, this message translates to:
  /// **'最近的运行日志'**
  String get diagRecent;

  /// No description provided for @diagLive.
  ///
  /// In zh, this message translates to:
  /// **'实时'**
  String get diagLive;

  /// No description provided for @diagNotConnected.
  ///
  /// In zh, this message translates to:
  /// **'未连接'**
  String get diagNotConnected;

  /// No description provided for @diagLevel.
  ///
  /// In zh, this message translates to:
  /// **'日志级别'**
  String get diagLevel;

  /// No description provided for @diagProblemsOnly.
  ///
  /// In zh, this message translates to:
  /// **'警告和错误'**
  String get diagProblemsOnly;

  /// No description provided for @diagNoProblems.
  ///
  /// In zh, this message translates to:
  /// **'最近没有警告和错误'**
  String get diagNoProblems;

  /// No description provided for @diagNoLog.
  ///
  /// In zh, this message translates to:
  /// **'还没有运行日志'**
  String get diagNoLog;

  /// No description provided for @diagLevelDebug.
  ///
  /// In zh, this message translates to:
  /// **'调试'**
  String get diagLevelDebug;

  /// No description provided for @diagLevelInfo.
  ///
  /// In zh, this message translates to:
  /// **'信息'**
  String get diagLevelInfo;

  /// No description provided for @diagLevelWarn.
  ///
  /// In zh, this message translates to:
  /// **'警告'**
  String get diagLevelWarn;

  /// No description provided for @diagLevelError.
  ///
  /// In zh, this message translates to:
  /// **'错误'**
  String get diagLevelError;

  /// No description provided for @diagCopied.
  ///
  /// In zh, this message translates to:
  /// **'已复制摘要：版本、系统和最近 200 行，可以直接贴进问题描述'**
  String get diagCopied;

  /// No description provided for @keyMapTitle.
  ///
  /// In zh, this message translates to:
  /// **'添加按键映射'**
  String get keyMapTitle;

  /// No description provided for @keyMapNothing.
  ///
  /// In zh, this message translates to:
  /// **'不发送'**
  String get keyMapNothing;

  /// No description provided for @keyMapOnlyFor.
  ///
  /// In zh, this message translates to:
  /// **'只对 {name} 生效'**
  String keyMapOnlyFor(String name);

  /// No description provided for @keyMapStep1.
  ///
  /// In zh, this message translates to:
  /// **'① 本机按哪个键'**
  String get keyMapStep1;

  /// No description provided for @keyMapRecording.
  ///
  /// In zh, this message translates to:
  /// **'正在录制，按一个键'**
  String get keyMapRecording;

  /// No description provided for @keyMapRerecord.
  ///
  /// In zh, this message translates to:
  /// **'重新录制'**
  String get keyMapRerecord;

  /// No description provided for @keyMapPressAKey.
  ///
  /// In zh, this message translates to:
  /// **'请在键盘上按一个键…'**
  String get keyMapPressAKey;

  /// No description provided for @keyMapEscCancels.
  ///
  /// In zh, this message translates to:
  /// **'Esc 取消'**
  String get keyMapEscCancels;

  /// No description provided for @keyMapRecorded.
  ///
  /// In zh, this message translates to:
  /// **'已录下'**
  String get keyMapRecorded;

  /// No description provided for @keyMapClickToRecord.
  ///
  /// In zh, this message translates to:
  /// **'点这里，再按一个键'**
  String get keyMapClickToRecord;

  /// No description provided for @keyMapModifiersToo.
  ///
  /// In zh, this message translates to:
  /// **'也能录单个修饰键，左右分开算。'**
  String get keyMapModifiersToo;

  /// No description provided for @keyMapStep2.
  ///
  /// In zh, this message translates to:
  /// **'② {name} 收到什么'**
  String keyMapStep2(String name);

  /// No description provided for @keyMapSearch.
  ///
  /// In zh, this message translates to:
  /// **'搜索按键，例如 Home、F5、音量'**
  String get keyMapSearch;

  /// No description provided for @keyMapPress.
  ///
  /// In zh, this message translates to:
  /// **'按'**
  String get keyMapPress;

  /// No description provided for @keyMapGetsNothing.
  ///
  /// In zh, this message translates to:
  /// **'{name} 什么也收不到'**
  String keyMapGetsNothing(String name);

  /// No description provided for @keyMapGets.
  ///
  /// In zh, this message translates to:
  /// **'{name} 收到'**
  String keyMapGets(String name);

  /// No description provided for @keyMapAdd.
  ///
  /// In zh, this message translates to:
  /// **'添加'**
  String get keyMapAdd;

  /// No description provided for @sideLeft.
  ///
  /// In zh, this message translates to:
  /// **'左侧'**
  String get sideLeft;

  /// No description provided for @sideRight.
  ///
  /// In zh, this message translates to:
  /// **'右侧'**
  String get sideRight;

  /// No description provided for @sideTop.
  ///
  /// In zh, this message translates to:
  /// **'上方'**
  String get sideTop;

  /// No description provided for @sideBottom.
  ///
  /// In zh, this message translates to:
  /// **'下方'**
  String get sideBottom;

  /// No description provided for @peerFallback.
  ///
  /// In zh, this message translates to:
  /// **'对方'**
  String get peerFallback;

  /// No description provided for @toastLinkCopied.
  ///
  /// In zh, this message translates to:
  /// **'打不开浏览器，链接已复制'**
  String get toastLinkCopied;

  /// No description provided for @toastNotConnected.
  ///
  /// In zh, this message translates to:
  /// **'还没连上后台，稍后再试'**
  String get toastNotConnected;

  /// No description provided for @toastConnectingTo.
  ///
  /// In zh, this message translates to:
  /// **'正在连接 {name}…'**
  String toastConnectingTo(String name);

  /// No description provided for @timeJustNow.
  ///
  /// In zh, this message translates to:
  /// **'刚刚'**
  String get timeJustNow;

  /// No description provided for @timeSecondsAgo.
  ///
  /// In zh, this message translates to:
  /// **'{n} 秒前'**
  String timeSecondsAgo(int n);

  /// No description provided for @timeMinutesAgo.
  ///
  /// In zh, this message translates to:
  /// **'{n} 分钟前'**
  String timeMinutesAgo(int n);

  /// No description provided for @timeHoursAgo.
  ///
  /// In zh, this message translates to:
  /// **'{n} 小时前'**
  String timeHoursAgo(int n);

  /// No description provided for @timeDaysAgo.
  ///
  /// In zh, this message translates to:
  /// **'{n} 天前'**
  String timeDaysAgo(int n);

  /// No description provided for @seenOnline.
  ///
  /// In zh, this message translates to:
  /// **'在线'**
  String get seenOnline;

  /// No description provided for @seenOffline.
  ///
  /// In zh, this message translates to:
  /// **'离线'**
  String get seenOffline;

  /// No description provided for @seenAgo.
  ///
  /// In zh, this message translates to:
  /// **'{ago}连接过'**
  String seenAgo(String ago);

  /// No description provided for @durationUnderMinute.
  ///
  /// In zh, this message translates to:
  /// **'不到 1 分钟'**
  String get durationUnderMinute;

  /// No description provided for @durationMinutes.
  ///
  /// In zh, this message translates to:
  /// **'{m} 分钟'**
  String durationMinutes(int m);

  /// No description provided for @durationHours.
  ///
  /// In zh, this message translates to:
  /// **'{h} 小时 {m} 分'**
  String durationHours(int h, int m);

  /// No description provided for @keyMedia.
  ///
  /// In zh, this message translates to:
  /// **'媒体键 0x{hex}'**
  String keyMedia(String hex);

  /// No description provided for @keyUnknown.
  ///
  /// In zh, this message translates to:
  /// **'按键 0x{hex}'**
  String keyUnknown(String hex);

  /// No description provided for @keyNumpad.
  ///
  /// In zh, this message translates to:
  /// **'小键盘 {key}'**
  String keyNumpad(String key);

  /// No description provided for @keyLeft.
  ///
  /// In zh, this message translates to:
  /// **'左 {key}'**
  String keyLeft(String key);

  /// No description provided for @keyRight.
  ///
  /// In zh, this message translates to:
  /// **'右 {key}'**
  String keyRight(String key);

  /// No description provided for @keySpace.
  ///
  /// In zh, this message translates to:
  /// **'空格'**
  String get keySpace;

  /// No description provided for @keyIso.
  ///
  /// In zh, this message translates to:
  /// **'\\（ISO）'**
  String get keyIso;

  /// No description provided for @keyMenu.
  ///
  /// In zh, this message translates to:
  /// **'菜单键'**
  String get keyMenu;

  /// No description provided for @keyMute.
  ///
  /// In zh, this message translates to:
  /// **'静音'**
  String get keyMute;

  /// No description provided for @keyVolumeUp.
  ///
  /// In zh, this message translates to:
  /// **'音量 +'**
  String get keyVolumeUp;

  /// No description provided for @keyVolumeDown.
  ///
  /// In zh, this message translates to:
  /// **'音量 −'**
  String get keyVolumeDown;

  /// No description provided for @keyHangul.
  ///
  /// In zh, this message translates to:
  /// **'韩/英、かな'**
  String get keyHangul;

  /// No description provided for @keyHanja.
  ///
  /// In zh, this message translates to:
  /// **'汉字、英数'**
  String get keyHanja;

  /// No description provided for @keyForwardDelete.
  ///
  /// In zh, this message translates to:
  /// **'⌦ 向前删除'**
  String get keyForwardDelete;

  /// No description provided for @keyBrightnessUp.
  ///
  /// In zh, this message translates to:
  /// **'亮度 +'**
  String get keyBrightnessUp;

  /// No description provided for @keyBrightnessDown.
  ///
  /// In zh, this message translates to:
  /// **'亮度 −'**
  String get keyBrightnessDown;

  /// No description provided for @keyNextTrack.
  ///
  /// In zh, this message translates to:
  /// **'下一首'**
  String get keyNextTrack;

  /// No description provided for @keyPreviousTrack.
  ///
  /// In zh, this message translates to:
  /// **'上一首'**
  String get keyPreviousTrack;

  /// No description provided for @keyStop.
  ///
  /// In zh, this message translates to:
  /// **'停止'**
  String get keyStop;

  /// No description provided for @keyPlayPause.
  ///
  /// In zh, this message translates to:
  /// **'播放/暂停'**
  String get keyPlayPause;

  /// No description provided for @settingsThisComputer.
  ///
  /// In zh, this message translates to:
  /// **'这台电脑'**
  String get settingsThisComputer;

  /// No description provided for @settingsRole.
  ///
  /// In zh, this message translates to:
  /// **'角色'**
  String get settingsRole;

  /// No description provided for @settingsRoleServerHint.
  ///
  /// In zh, this message translates to:
  /// **'键盘鼠标接在这台上，用它控制别的电脑'**
  String get settingsRoleServerHint;

  /// No description provided for @settingsRoleClientHint.
  ///
  /// In zh, this message translates to:
  /// **'由另一台电脑的键盘鼠标控制 · 切换后重新连接'**
  String get settingsRoleClientHint;

  /// No description provided for @settingsRoleServer.
  ///
  /// In zh, this message translates to:
  /// **'控制别的电脑'**
  String get settingsRoleServer;

  /// No description provided for @settingsRoleClient.
  ///
  /// In zh, this message translates to:
  /// **'被控制'**
  String get settingsRoleClient;

  /// No description provided for @settingsAppearance.
  ///
  /// In zh, this message translates to:
  /// **'外观'**
  String get settingsAppearance;

  /// No description provided for @settingsTheme.
  ///
  /// In zh, this message translates to:
  /// **'主题'**
  String get settingsTheme;

  /// No description provided for @settingsFollowSystem.
  ///
  /// In zh, this message translates to:
  /// **'跟随系统'**
  String get settingsFollowSystem;

  /// No description provided for @settingsLight.
  ///
  /// In zh, this message translates to:
  /// **'浅色'**
  String get settingsLight;

  /// No description provided for @settingsDark.
  ///
  /// In zh, this message translates to:
  /// **'深色'**
  String get settingsDark;

  /// No description provided for @settingsLanguage.
  ///
  /// In zh, this message translates to:
  /// **'语言'**
  String get settingsLanguage;

  /// No description provided for @settingsAccent.
  ///
  /// In zh, this message translates to:
  /// **'强调色'**
  String get settingsAccent;

  /// No description provided for @settingsAccentHint.
  ///
  /// In zh, this message translates to:
  /// **'开关、选中项和按钮的颜色'**
  String get settingsAccentHint;

  /// No description provided for @settingsMouseClipboard.
  ///
  /// In zh, this message translates to:
  /// **'鼠标与剪贴板'**
  String get settingsMouseClipboard;

  /// No description provided for @settingsDpi.
  ///
  /// In zh, this message translates to:
  /// **'本机鼠标的 DPI'**
  String get settingsDpi;

  /// No description provided for @settingsDpiHint.
  ///
  /// In zh, this message translates to:
  /// **'每台设备的光标速度以它为准'**
  String get settingsDpiHint;

  /// No description provided for @settingsPerDevice.
  ///
  /// In zh, this message translates to:
  /// **'按键映射、滚动方向'**
  String get settingsPerDevice;

  /// No description provided for @settingsPerDeviceHint.
  ///
  /// In zh, this message translates to:
  /// **'在“设备”里逐台设置'**
  String get settingsPerDeviceHint;

  /// No description provided for @settingsClipboard.
  ///
  /// In zh, this message translates to:
  /// **'同步剪贴板'**
  String get settingsClipboard;

  /// No description provided for @settingsClipboardHint.
  ///
  /// In zh, this message translates to:
  /// **'光标切过去时，对方才来取最新的内容'**
  String get settingsClipboardHint;

  /// No description provided for @settingsClipLimit.
  ///
  /// In zh, this message translates to:
  /// **'单项大小上限'**
  String get settingsClipLimit;

  /// No description provided for @settingsClipLimitHint.
  ///
  /// In zh, this message translates to:
  /// **'图片、文字等每一项超过这么大就不同步'**
  String get settingsClipLimitHint;

  /// No description provided for @settingsGeneral.
  ///
  /// In zh, this message translates to:
  /// **'通用'**
  String get settingsGeneral;

  /// No description provided for @settingsAutostart.
  ///
  /// In zh, this message translates to:
  /// **'开机自动启动'**
  String get settingsAutostart;

  /// No description provided for @settingsAnnounce.
  ///
  /// In zh, this message translates to:
  /// **'在局域网里广播本机'**
  String get settingsAnnounce;

  /// No description provided for @settingsAnnounceHint.
  ///
  /// In zh, this message translates to:
  /// **'另一台电脑不用填地址就能找到本机'**
  String get settingsAnnounceHint;

  /// No description provided for @settingsDataDir.
  ///
  /// In zh, this message translates to:
  /// **'数据目录'**
  String get settingsDataDir;

  /// No description provided for @settingsAbout.
  ///
  /// In zh, this message translates to:
  /// **'关于 ScreenFerry'**
  String get settingsAbout;

  /// No description provided for @settingsVersion.
  ///
  /// In zh, this message translates to:
  /// **'版本 {version}'**
  String settingsVersion(String version);

  /// 并列的短句之间
  ///
  /// In zh, this message translates to:
  /// **'，'**
  String get commonComma;

  /// 并列的词之间
  ///
  /// In zh, this message translates to:
  /// **'、'**
  String get commonEnum;

  /// No description provided for @commonConnected.
  ///
  /// In zh, this message translates to:
  /// **'已连接'**
  String get commonConnected;

  /// No description provided for @commonNotConnected.
  ///
  /// In zh, this message translates to:
  /// **'未连接'**
  String get commonNotConnected;

  /// No description provided for @commonThisComputer.
  ///
  /// In zh, this message translates to:
  /// **'本机'**
  String get commonThisComputer;

  /// No description provided for @layoutTitle.
  ///
  /// In zh, this message translates to:
  /// **'布局'**
  String get layoutTitle;

  /// No description provided for @layoutSetOnServer.
  ///
  /// In zh, this message translates to:
  /// **'屏幕位置在 {name} 上设置'**
  String layoutSetOnServer(String name);

  /// No description provided for @layoutSetOnServerHint.
  ///
  /// In zh, this message translates to:
  /// **'键盘鼠标在那台电脑上，由它决定从哪条边切过来。'**
  String get layoutSetOnServerHint;

  /// No description provided for @layoutSwitchMode.
  ///
  /// In zh, this message translates to:
  /// **'切换方式'**
  String get layoutSwitchMode;

  /// No description provided for @layoutSwitchModeHint.
  ///
  /// In zh, this message translates to:
  /// **'停留一下再切换，可以避免鼠标甩到边缘时误切 · 即将支持'**
  String get layoutSwitchModeHint;

  /// No description provided for @layoutInstant.
  ///
  /// In zh, this message translates to:
  /// **'立即'**
  String get layoutInstant;

  /// No description provided for @layoutDwell.
  ///
  /// In zh, this message translates to:
  /// **'停留 0.15 秒'**
  String get layoutDwell;

  /// No description provided for @layoutNoDragSwitch.
  ///
  /// In zh, this message translates to:
  /// **'拖拽时不切换'**
  String get layoutNoDragSwitch;

  /// No description provided for @layoutNoDragSwitchHint.
  ///
  /// In zh, this message translates to:
  /// **'按住鼠标键时，光标不会切到另一台电脑 · 目前始终开启'**
  String get layoutNoDragSwitchHint;

  /// No description provided for @layoutBackHere.
  ///
  /// In zh, this message translates to:
  /// **'回到本机'**
  String get layoutBackHere;

  /// No description provided for @layoutBackHereHint.
  ///
  /// In zh, this message translates to:
  /// **'光标卡在对方屏幕上时，按它立即回来'**
  String get layoutBackHereHint;

  /// No description provided for @layoutGoPair.
  ///
  /// In zh, this message translates to:
  /// **'去配对'**
  String get layoutGoPair;

  /// No description provided for @layoutWhere.
  ///
  /// In zh, this message translates to:
  /// **'本机{side, select, left{左侧} right{右侧} top{上方} other{下方}}'**
  String layoutWhere(String side);

  /// No description provided for @layoutWhereNth.
  ///
  /// In zh, this message translates to:
  /// **'本机{side, select, left{左侧从上往下} right{右侧从上往下} top{上方从左往右} other{下方从左往右}}第 {n} 台'**
  String layoutWhereNth(String side, int n);

  /// No description provided for @layoutPeerAt.
  ///
  /// In zh, this message translates to:
  /// **'{name}，在{where}'**
  String layoutPeerAt(String name, String where);

  /// No description provided for @layoutCursorOnIt.
  ///
  /// In zh, this message translates to:
  /// **'光标在这台上'**
  String get layoutCursorOnIt;

  /// No description provided for @layoutMovable.
  ///
  /// In zh, this message translates to:
  /// **'可以拖动'**
  String get layoutMovable;

  /// No description provided for @layoutEmpty.
  ///
  /// In zh, this message translates to:
  /// **'还没有配对的设备。在另一台电脑上运行 ScreenFerry 并配对后，它会出现在本机旁边'**
  String get layoutEmpty;

  /// No description provided for @layoutDropAt.
  ///
  /// In zh, this message translates to:
  /// **'松手后放到{where}'**
  String layoutDropAt(String where);

  /// No description provided for @layoutSingle.
  ///
  /// In zh, this message translates to:
  /// **'{name} 在{where}，光标从相接的那条边穿过去'**
  String layoutSingle(String name, String where);

  /// No description provided for @layoutSideCount.
  ///
  /// In zh, this message translates to:
  /// **'{side, select, left{左侧} right{右侧} top{上方} other{下方}} {n} 台'**
  String layoutSideCount(String side, int n);

  /// No description provided for @layoutMany.
  ///
  /// In zh, this message translates to:
  /// **'本机{parts}。同一侧已连接的几台按先后平分那条边，光标从哪一段穿过去就到哪台'**
  String layoutMany(String parts);

  /// No description provided for @layoutCursorHere.
  ///
  /// In zh, this message translates to:
  /// **'光标在这里'**
  String get layoutCursorHere;

  /// No description provided for @devicesThisName.
  ///
  /// In zh, this message translates to:
  /// **'本机的名字'**
  String get devicesThisName;

  /// No description provided for @devicesComputerName.
  ///
  /// In zh, this message translates to:
  /// **'电脑名'**
  String get devicesComputerName;

  /// No description provided for @devicesNameHint.
  ///
  /// In zh, this message translates to:
  /// **'别的电脑上看到的就是这个名字。留空就用电脑名。'**
  String get devicesNameHint;

  /// No description provided for @devicesThisFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'本机 · 指纹 '**
  String get devicesThisFingerprint;

  /// No description provided for @devicesCopyFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'复制指纹'**
  String get devicesCopyFingerprint;

  /// No description provided for @devicesFingerprintCopied.
  ///
  /// In zh, this message translates to:
  /// **'指纹已复制'**
  String get devicesFingerprintCopied;

  /// No description provided for @devicesPaired.
  ///
  /// In zh, this message translates to:
  /// **'已配对'**
  String get devicesPaired;

  /// No description provided for @devicesCount.
  ///
  /// In zh, this message translates to:
  /// **'{n} 台'**
  String devicesCount(int n);

  /// No description provided for @devicesNonePaired.
  ///
  /// In zh, this message translates to:
  /// **'还没有配对的设备'**
  String get devicesNonePaired;

  /// No description provided for @devicesNonePairedHint.
  ///
  /// In zh, this message translates to:
  /// **'在另一台电脑上运行 ScreenFerry，两边核对配对码后会出现在这里。'**
  String get devicesNonePairedHint;

  /// No description provided for @devicesFound.
  ///
  /// In zh, this message translates to:
  /// **'在局域网里发现'**
  String get devicesFound;

  /// No description provided for @devicesNotPairedYet.
  ///
  /// In zh, this message translates to:
  /// **'还没配对'**
  String get devicesNotPairedYet;

  /// No description provided for @devicesInvite.
  ///
  /// In zh, this message translates to:
  /// **'邀请一台电脑'**
  String get devicesInvite;

  /// No description provided for @devicesConnectByAddress.
  ///
  /// In zh, this message translates to:
  /// **'按地址连接'**
  String get devicesConnectByAddress;

  /// No description provided for @devicesWaiting.
  ///
  /// In zh, this message translates to:
  /// **'等另一台电脑来找本机'**
  String get devicesWaiting;

  /// No description provided for @devicesWaitingHint.
  ///
  /// In zh, this message translates to:
  /// **'在另一台电脑上运行 ScreenFerry 并选择本机后，这里会弹出配对码请你核对。'**
  String get devicesWaitingHint;

  /// No description provided for @devicesWaitingHintAddress.
  ///
  /// In zh, this message translates to:
  /// **'它在局域网里找不到本机时，在那台电脑上按地址连接 {address}；那台电脑没有键盘鼠标时，在这里邀请它连过来。'**
  String devicesWaitingHintAddress(String address);

  /// No description provided for @devicesNoneFound.
  ///
  /// In zh, this message translates to:
  /// **'局域网里没有找到其他设备'**
  String get devicesNoneFound;

  /// No description provided for @devicesNoneFoundHint.
  ///
  /// In zh, this message translates to:
  /// **'有的网络不支持自动发现（例如公司网络、开了设备隔离的 Wi‑Fi）。可以按地址连接，地址在控制端的「设备」页上。'**
  String get devicesNoneFoundHint;

  /// No description provided for @devicesThisAddress.
  ///
  /// In zh, this message translates to:
  /// **'本机地址'**
  String get devicesThisAddress;

  /// No description provided for @devicesAddressCopied.
  ///
  /// In zh, this message translates to:
  /// **'地址已复制'**
  String get devicesAddressCopied;

  /// No description provided for @devicesCopy.
  ///
  /// In zh, this message translates to:
  /// **'复制 {address}'**
  String devicesCopy(String address);

  /// No description provided for @devicesCopyAddress.
  ///
  /// In zh, this message translates to:
  /// **'复制本机地址 {address}'**
  String devicesCopyAddress(String address);

  /// No description provided for @devicesOnlineAt.
  ///
  /// In zh, this message translates to:
  /// **'在线 · {address}'**
  String devicesOnlineAt(String address);

  /// No description provided for @devicesSettingsOf.
  ///
  /// In zh, this message translates to:
  /// **'{name} 的设置'**
  String devicesSettingsOf(String name);

  /// No description provided for @devicesFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'指纹 '**
  String get devicesFingerprint;

  /// No description provided for @devicesPlace.
  ///
  /// In zh, this message translates to:
  /// **'在本机{side, select, left{左侧} right{右侧} top{上方} other{下方}}'**
  String devicesPlace(String side);

  /// No description provided for @devicesSettings.
  ///
  /// In zh, this message translates to:
  /// **'设置'**
  String get devicesSettings;

  /// No description provided for @devicesNotPaired.
  ///
  /// In zh, this message translates to:
  /// **'未配对'**
  String get devicesNotPaired;

  /// No description provided for @devicesFingerprintShort.
  ///
  /// In zh, this message translates to:
  /// **'指纹 {short}'**
  String devicesFingerprintShort(String short);

  /// No description provided for @devicesAnnouncing.
  ///
  /// In zh, this message translates to:
  /// **'正在广播'**
  String get devicesAnnouncing;

  /// No description provided for @devicesPair.
  ///
  /// In zh, this message translates to:
  /// **'配对'**
  String get devicesPair;

  /// No description provided for @overviewTitle.
  ///
  /// In zh, this message translates to:
  /// **'仪表盘'**
  String get overviewTitle;

  /// No description provided for @overviewSharing.
  ///
  /// In zh, this message translates to:
  /// **'共享中'**
  String get overviewSharing;

  /// No description provided for @overviewPausedTitle.
  ///
  /// In zh, this message translates to:
  /// **'已暂停共享'**
  String get overviewPausedTitle;

  /// No description provided for @overviewPaused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停'**
  String get overviewPaused;

  /// No description provided for @overviewWaitingTitle.
  ///
  /// In zh, this message translates to:
  /// **'等待对方连接'**
  String get overviewWaitingTitle;

  /// No description provided for @overviewWaiting.
  ///
  /// In zh, this message translates to:
  /// **'等待连接'**
  String get overviewWaiting;

  /// No description provided for @overviewSearchingTitle.
  ///
  /// In zh, this message translates to:
  /// **'正在寻找对方'**
  String get overviewSearchingTitle;

  /// No description provided for @overviewSearching.
  ///
  /// In zh, this message translates to:
  /// **'寻找中'**
  String get overviewSearching;

  /// No description provided for @overviewConnectingTitle.
  ///
  /// In zh, this message translates to:
  /// **'正在连接…'**
  String get overviewConnectingTitle;

  /// No description provided for @overviewConnecting.
  ///
  /// In zh, this message translates to:
  /// **'连接中'**
  String get overviewConnecting;

  /// No description provided for @overviewStartingTitle.
  ///
  /// In zh, this message translates to:
  /// **'正在启动…'**
  String get overviewStartingTitle;

  /// No description provided for @overviewStarting.
  ///
  /// In zh, this message translates to:
  /// **'启动中'**
  String get overviewStarting;

  /// No description provided for @overviewStoppedTitle.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法共享'**
  String get overviewStoppedTitle;

  /// No description provided for @overviewStopped.
  ///
  /// In zh, this message translates to:
  /// **'已停止'**
  String get overviewStopped;

  /// No description provided for @overviewPausedHint.
  ///
  /// In zh, this message translates to:
  /// **'键鼠留在本机。点右边的按钮继续共享'**
  String get overviewPausedHint;

  /// No description provided for @overviewAnnouncingHint.
  ///
  /// In zh, this message translates to:
  /// **'已在局域网里广播本机，另一台电脑会自动找到这里'**
  String get overviewAnnouncingHint;

  /// No description provided for @overviewNotAnnouncingHint.
  ///
  /// In zh, this message translates to:
  /// **'没有广播本机，另一台电脑需要手动填写本机地址'**
  String get overviewNotAnnouncingHint;

  /// No description provided for @overviewSearchingHint.
  ///
  /// In zh, this message translates to:
  /// **'在局域网里寻找已配对的电脑'**
  String get overviewSearchingHint;

  /// No description provided for @overviewConnectingHint.
  ///
  /// In zh, this message translates to:
  /// **'找到了对方，正在建立加密连接'**
  String get overviewConnectingHint;

  /// No description provided for @overviewStartingHint.
  ///
  /// In zh, this message translates to:
  /// **'稍等片刻'**
  String get overviewStartingHint;

  /// No description provided for @overviewProblemHint.
  ///
  /// In zh, this message translates to:
  /// **'后台遇到了问题'**
  String get overviewProblemHint;

  /// No description provided for @overviewStatus.
  ///
  /// In zh, this message translates to:
  /// **'状态'**
  String get overviewStatus;

  /// No description provided for @overviewLatencyGreat.
  ///
  /// In zh, this message translates to:
  /// **'延迟 优'**
  String get overviewLatencyGreat;

  /// No description provided for @overviewLatencyGood.
  ///
  /// In zh, this message translates to:
  /// **'延迟 良好'**
  String get overviewLatencyGood;

  /// No description provided for @overviewLatencyHigh.
  ///
  /// In zh, this message translates to:
  /// **'延迟 较高'**
  String get overviewLatencyHigh;

  /// No description provided for @overviewCursorOn.
  ///
  /// In zh, this message translates to:
  /// **'光标在 {name}'**
  String overviewCursorOn(String name);

  /// No description provided for @overviewCursorHere.
  ///
  /// In zh, this message translates to:
  /// **'光标在本机'**
  String get overviewCursorHere;

  /// No description provided for @overviewConnectedCount.
  ///
  /// In zh, this message translates to:
  /// **'已连接 {n} 台'**
  String overviewConnectedCount(int n);

  /// No description provided for @overviewAddress.
  ///
  /// In zh, this message translates to:
  /// **'地址'**
  String get overviewAddress;

  /// No description provided for @overviewEncryption.
  ///
  /// In zh, this message translates to:
  /// **'加密'**
  String get overviewEncryption;

  /// No description provided for @overviewFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'指纹'**
  String get overviewFingerprint;

  /// No description provided for @overviewPause.
  ///
  /// In zh, this message translates to:
  /// **'暂停共享'**
  String get overviewPause;

  /// No description provided for @overviewResume.
  ///
  /// In zh, this message translates to:
  /// **'继续共享'**
  String get overviewResume;

  /// No description provided for @overviewRtt.
  ///
  /// In zh, this message translates to:
  /// **'往返延迟'**
  String get overviewRtt;

  /// No description provided for @overviewPeak.
  ///
  /// In zh, this message translates to:
  /// **'峰值 {ms}'**
  String overviewPeak(String ms);

  /// No description provided for @overviewRttEmpty.
  ///
  /// In zh, this message translates to:
  /// **'连上对方后显示'**
  String get overviewRttEmpty;

  /// No description provided for @overviewRttAverage.
  ///
  /// In zh, this message translates to:
  /// **'最近 {n} 秒平均 {ms} ms'**
  String overviewRttAverage(int n, String ms);

  /// No description provided for @overviewClipboard.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板'**
  String get overviewClipboard;

  /// No description provided for @overviewNoClip.
  ///
  /// In zh, this message translates to:
  /// **'还没有同步过剪贴板'**
  String get overviewNoClip;

  /// No description provided for @overviewNoClipHint.
  ///
  /// In zh, this message translates to:
  /// **'复制后把光标切过去，对方就能直接粘贴。'**
  String get overviewNoClipHint;

  /// No description provided for @overviewClipOffHint.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板同步已关闭，可以在设置里打开。'**
  String get overviewClipOffHint;

  /// No description provided for @overviewClipImage.
  ///
  /// In zh, this message translates to:
  /// **'图片 · {width}×{height} · {size}'**
  String overviewClipImage(int width, int height, String size);

  /// No description provided for @overviewClipRich.
  ///
  /// In zh, this message translates to:
  /// **'带格式的文本 · {n} 字'**
  String overviewClipRich(int n);

  /// No description provided for @overviewClipText.
  ///
  /// In zh, this message translates to:
  /// **'文本 · {n} 字'**
  String overviewClipText(int n);

  /// No description provided for @overviewAnImage.
  ///
  /// In zh, this message translates to:
  /// **'一张图片'**
  String get overviewAnImage;

  /// No description provided for @overviewNotText.
  ///
  /// In zh, this message translates to:
  /// **'（不是文字）'**
  String get overviewNotText;

  /// No description provided for @overviewClipImageLabel.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板图片 {width}×{height}'**
  String overviewClipImageLabel(int width, int height);

  /// No description provided for @overviewClipSync.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板同步'**
  String get overviewClipSync;

  /// No description provided for @overviewAutostart.
  ///
  /// In zh, this message translates to:
  /// **'开机自动启动'**
  String get overviewAutostart;

  /// No description provided for @overviewAnnounce.
  ///
  /// In zh, this message translates to:
  /// **'局域网广播'**
  String get overviewAnnounce;

  /// No description provided for @overviewOn.
  ///
  /// In zh, this message translates to:
  /// **'已开启'**
  String get overviewOn;

  /// No description provided for @overviewOff.
  ///
  /// In zh, this message translates to:
  /// **'已关闭'**
  String get overviewOff;

  /// No description provided for @overviewManage.
  ///
  /// In zh, this message translates to:
  /// **'管理'**
  String get overviewManage;

  /// No description provided for @overviewFoundNew.
  ///
  /// In zh, this message translates to:
  /// **'局域网里发现 {n} 台新设备'**
  String overviewFoundNew(int n);

  /// No description provided for @overviewThisName.
  ///
  /// In zh, this message translates to:
  /// **'{name} · 本机'**
  String overviewThisName(String name);

  /// No description provided for @overviewCopyFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'复制本机指纹'**
  String get overviewCopyFingerprint;

  /// No description provided for @overviewToday.
  ///
  /// In zh, this message translates to:
  /// **'今日'**
  String get overviewToday;

  /// No description provided for @overviewSwitches.
  ///
  /// In zh, this message translates to:
  /// **'次屏幕切换'**
  String overviewSwitches(int n);

  /// No description provided for @overviewClips.
  ///
  /// In zh, this message translates to:
  /// **'次剪贴板同步'**
  String overviewClips(int n);

  /// No description provided for @overviewHours.
  ///
  /// In zh, this message translates to:
  /// **'小时连接'**
  String get overviewHours;

  /// No description provided for @deviceLater.
  ///
  /// In zh, this message translates to:
  /// **'本机控制它时生效'**
  String get deviceLater;

  /// No description provided for @deviceBack.
  ///
  /// In zh, this message translates to:
  /// **'返回设备列表'**
  String get deviceBack;

  /// No description provided for @deviceMouse.
  ///
  /// In zh, this message translates to:
  /// **'鼠标'**
  String get deviceMouse;

  /// No description provided for @deviceInvertVertical.
  ///
  /// In zh, this message translates to:
  /// **'垂直滚动反向'**
  String get deviceInvertVertical;

  /// No description provided for @deviceInvertVerticalHint.
  ///
  /// In zh, this message translates to:
  /// **'上下滚动的方向反过来'**
  String get deviceInvertVerticalHint;

  /// No description provided for @deviceInvertHorizontal.
  ///
  /// In zh, this message translates to:
  /// **'水平滚动反向'**
  String get deviceInvertHorizontal;

  /// No description provided for @deviceInvertHorizontalHint.
  ///
  /// In zh, this message translates to:
  /// **'左右滚动的方向反过来'**
  String get deviceInvertHorizontalHint;

  /// No description provided for @deviceScrollSpeed.
  ///
  /// In zh, this message translates to:
  /// **'滚动速度'**
  String get deviceScrollSpeed;

  /// No description provided for @deviceScrollSpeedHint.
  ///
  /// In zh, this message translates to:
  /// **'触控板的细小滚动到了这台上偏慢或偏快时调'**
  String get deviceScrollSpeedHint;

  /// No description provided for @deviceSlow.
  ///
  /// In zh, this message translates to:
  /// **'慢'**
  String get deviceSlow;

  /// No description provided for @deviceNormal.
  ///
  /// In zh, this message translates to:
  /// **'标准'**
  String get deviceNormal;

  /// No description provided for @deviceFast.
  ///
  /// In zh, this message translates to:
  /// **'快'**
  String get deviceFast;

  /// No description provided for @deviceNaturalTip.
  ///
  /// In zh, this message translates to:
  /// **'本机开着“自然滚动”，在这台上也按自然方向滚。想和普通鼠标一样，打开“垂直滚动反向”。'**
  String get deviceNaturalTip;

  /// No description provided for @deviceKeys.
  ///
  /// In zh, this message translates to:
  /// **'按键'**
  String get deviceKeys;

  /// No description provided for @deviceModifiers.
  ///
  /// In zh, this message translates to:
  /// **'修饰键'**
  String get deviceModifiers;

  /// No description provided for @deviceModifierPreset.
  ///
  /// In zh, this message translates to:
  /// **'修饰键方案'**
  String get deviceModifierPreset;

  /// No description provided for @deviceMacHabit.
  ///
  /// In zh, this message translates to:
  /// **'Mac 习惯'**
  String get deviceMacHabit;

  /// No description provided for @deviceXkbHabit.
  ///
  /// In zh, this message translates to:
  /// **'PC / XKB 习惯'**
  String get deviceXkbHabit;

  /// No description provided for @deviceKeyboardUnknown.
  ///
  /// In zh, this message translates to:
  /// **'还不知道两台设备的键盘类型，修饰键先按原样发送。连上并交换系统信息后会自动调整。'**
  String get deviceKeyboardUnknown;

  /// No description provided for @deviceWindowsHabit.
  ///
  /// In zh, this message translates to:
  /// **'Windows 习惯'**
  String get deviceWindowsHabit;

  /// No description provided for @deviceAsIs.
  ///
  /// In zh, this message translates to:
  /// **'原样'**
  String get deviceAsIs;

  /// No description provided for @deviceCustom.
  ///
  /// In zh, this message translates to:
  /// **'自定义'**
  String get deviceCustom;

  /// No description provided for @deviceOtherKeys.
  ///
  /// In zh, this message translates to:
  /// **'其他按键映射'**
  String get deviceOtherKeys;

  /// No description provided for @deviceOtherKeysHint.
  ///
  /// In zh, this message translates to:
  /// **'把一个键换成另一个键，或者不发送'**
  String get deviceOtherKeysHint;

  /// No description provided for @deviceTextMac.
  ///
  /// In zh, this message translates to:
  /// **'文本编辑快捷键按 Mac 习惯'**
  String get deviceTextMac;

  /// No description provided for @deviceTextXkb.
  ///
  /// In zh, this message translates to:
  /// **'文本编辑快捷键按 PC / XKB 习惯'**
  String get deviceTextXkb;

  /// No description provided for @deviceTextWindows.
  ///
  /// In zh, this message translates to:
  /// **'文本编辑快捷键按 Windows 习惯'**
  String get deviceTextWindows;

  /// No description provided for @deviceTextMacHint.
  ///
  /// In zh, this message translates to:
  /// **'⌘← ⌘→ 到行首行尾，⌥← ⌥→ 按词移动，⌘⌫ 删到行首'**
  String get deviceTextMacHint;

  /// No description provided for @deviceTextXkbHint.
  ///
  /// In zh, this message translates to:
  /// **'Home End 到行首行尾，Ctrl+← Ctrl+→ 按词移动'**
  String get deviceTextXkbHint;

  /// No description provided for @deviceTextWindowsHint.
  ///
  /// In zh, this message translates to:
  /// **'Home End 到行首行尾，Ctrl+← Ctrl+→ 按词移动'**
  String get deviceTextWindowsHint;

  /// No description provided for @deviceTextShortcuts.
  ///
  /// In zh, this message translates to:
  /// **'文本编辑快捷键'**
  String get deviceTextShortcuts;

  /// No description provided for @deviceSwitching.
  ///
  /// In zh, this message translates to:
  /// **'切换与共享'**
  String get deviceSwitching;

  /// No description provided for @deviceDwell.
  ///
  /// In zh, this message translates to:
  /// **'防止误切过去'**
  String get deviceDwell;

  /// No description provided for @deviceDwellServerHint.
  ///
  /// In zh, this message translates to:
  /// **'光标在边上停一会儿才切到这台'**
  String get deviceDwellServerHint;

  /// No description provided for @deviceDwellClientHint.
  ///
  /// In zh, this message translates to:
  /// **'本机控制它时，光标在边上停一会儿才切过去'**
  String get deviceDwellClientHint;

  /// No description provided for @deviceDwellLabel.
  ///
  /// In zh, this message translates to:
  /// **'切过去前在边上停多久'**
  String get deviceDwellLabel;

  /// No description provided for @deviceOff.
  ///
  /// In zh, this message translates to:
  /// **'关'**
  String get deviceOff;

  /// No description provided for @deviceSeconds.
  ///
  /// In zh, this message translates to:
  /// **'{s} 秒'**
  String deviceSeconds(String s);

  /// No description provided for @deviceClipboardHint.
  ///
  /// In zh, this message translates to:
  /// **'和这台之间同步复制的内容'**
  String get deviceClipboardHint;

  /// No description provided for @deviceClipboardOffHint.
  ///
  /// In zh, this message translates to:
  /// **'“设置”里的剪贴板总开关关着'**
  String get deviceClipboardOffHint;

  /// No description provided for @deviceClipboardLabel.
  ///
  /// In zh, this message translates to:
  /// **'和这台同步剪贴板'**
  String get deviceClipboardLabel;

  /// No description provided for @deviceDropFiles.
  ///
  /// In zh, this message translates to:
  /// **'拖文件给它'**
  String get deviceDropFiles;

  /// No description provided for @deviceDropFilesHint.
  ///
  /// In zh, this message translates to:
  /// **'把文件拖到朝向它的屏幕边上就发过去'**
  String get deviceDropFilesHint;

  /// No description provided for @deviceConnection.
  ///
  /// In zh, this message translates to:
  /// **'连接'**
  String get deviceConnection;

  /// No description provided for @deviceThisDevice.
  ///
  /// In zh, this message translates to:
  /// **'这台设备'**
  String get deviceThisDevice;

  /// No description provided for @devicePresetMacDiffer.
  ///
  /// In zh, this message translates to:
  /// **'在 {name} 上按 ⌘C、⌘V 就是复制、粘贴'**
  String devicePresetMacDiffer(String name);

  /// No description provided for @devicePresetWinDiffer.
  ///
  /// In zh, this message translates to:
  /// **'在 {name} 上按 Ctrl+C、Ctrl+V 就是复制、粘贴'**
  String devicePresetWinDiffer(String name);

  /// No description provided for @devicePresetSame.
  ///
  /// In zh, this message translates to:
  /// **'两台是同一种键盘，原样发送'**
  String get devicePresetSame;

  /// No description provided for @devicePresetPlainMac.
  ///
  /// In zh, this message translates to:
  /// **'原样发送：⌘ 到了那边是 Win 键'**
  String get devicePresetPlainMac;

  /// No description provided for @devicePresetPlainWin.
  ///
  /// In zh, this message translates to:
  /// **'原样发送：Win 键到了那边是 ⌘'**
  String get devicePresetPlainWin;

  /// No description provided for @devicePresetPlain.
  ///
  /// In zh, this message translates to:
  /// **'原样发送'**
  String get devicePresetPlain;

  /// No description provided for @devicePresetCustom.
  ///
  /// In zh, this message translates to:
  /// **'按下面的对照表逐个对应'**
  String get devicePresetCustom;

  /// No description provided for @deviceAlias.
  ///
  /// In zh, this message translates to:
  /// **'在本机上叫它'**
  String get deviceAlias;

  /// No description provided for @deviceAliasHint.
  ///
  /// In zh, this message translates to:
  /// **'只改本机上显示的名字，那台电脑自己的名字不变。留空就用原名。'**
  String get deviceAliasHint;

  /// No description provided for @deviceOriginalName.
  ///
  /// In zh, this message translates to:
  /// **'原名 {name}'**
  String deviceOriginalName(String name);

  /// No description provided for @deviceSystemFingerprint.
  ///
  /// In zh, this message translates to:
  /// **'{system} · 指纹 '**
  String deviceSystemFingerprint(String system);

  /// No description provided for @deviceControlsThis.
  ///
  /// In zh, this message translates to:
  /// **'控制着本机'**
  String get deviceControlsThis;

  /// No description provided for @devicePlaceAdjust.
  ///
  /// In zh, this message translates to:
  /// **'在本机{side, select, left{左侧} right{右侧} top{上方} other{下方}} · 调整位置'**
  String devicePlaceAdjust(String side);

  /// No description provided for @deviceControlled.
  ///
  /// In zh, this message translates to:
  /// **'本机现在是被控制的一方'**
  String get deviceControlled;

  /// No description provided for @deviceControlledHint.
  ///
  /// In zh, this message translates to:
  /// **'鼠标、按键和防误切在本机控制它时才生效。被它控制时的滚动方向和按键，要在 {name} 上改。'**
  String deviceControlledHint(String name);

  /// No description provided for @deviceOpenOn.
  ///
  /// In zh, this message translates to:
  /// **'在 {name} 上打开'**
  String deviceOpenOn(String name);

  /// No description provided for @devicePointerSpeed.
  ///
  /// In zh, this message translates to:
  /// **'光标速度'**
  String get devicePointerSpeed;

  /// No description provided for @deviceBaseHere.
  ///
  /// In zh, this message translates to:
  /// **'{dpi} · 本机'**
  String deviceBaseHere(int dpi);

  /// No description provided for @deviceReset.
  ///
  /// In zh, this message translates to:
  /// **'还原'**
  String get deviceReset;

  /// No description provided for @deviceLocalPress.
  ///
  /// In zh, this message translates to:
  /// **'本机按'**
  String get deviceLocalPress;

  /// No description provided for @devicePeerGets.
  ///
  /// In zh, this message translates to:
  /// **'{name} 收到'**
  String devicePeerGets(String name);

  /// No description provided for @deviceModifierBecomes.
  ///
  /// In zh, this message translates to:
  /// **'{key} 在 {name} 上变成'**
  String deviceModifierBecomes(String key, String name);

  /// No description provided for @deviceDeleteMapping.
  ///
  /// In zh, this message translates to:
  /// **'删除这条映射'**
  String get deviceDeleteMapping;

  /// No description provided for @deviceLastAddress.
  ///
  /// In zh, this message translates to:
  /// **'上次连上的地址'**
  String get deviceLastAddress;

  /// No description provided for @deviceLastAddressServerHint.
  ///
  /// In zh, this message translates to:
  /// **'它上次从这个地址连过来'**
  String get deviceLastAddressServerHint;

  /// No description provided for @deviceLastAddressClientHint.
  ///
  /// In zh, this message translates to:
  /// **'局域网里找不到它时，先试这个地址'**
  String get deviceLastAddressClientHint;

  /// No description provided for @deviceNotYet.
  ///
  /// In zh, this message translates to:
  /// **'还没有'**
  String get deviceNotYet;

  /// No description provided for @deviceInvite.
  ///
  /// In zh, this message translates to:
  /// **'邀请它连过来'**
  String get deviceInvite;

  /// No description provided for @deviceInviteOnlineHint.
  ///
  /// In zh, this message translates to:
  /// **'它已经连着本机'**
  String get deviceInviteOnlineHint;

  /// No description provided for @deviceInviteHint.
  ///
  /// In zh, this message translates to:
  /// **'它在局域网里找不到本机、身边又没有键盘鼠标时用'**
  String get deviceInviteHint;

  /// No description provided for @deviceInviteButton.
  ///
  /// In zh, this message translates to:
  /// **'邀请'**
  String get deviceInviteButton;

  /// No description provided for @deviceConnectMode.
  ///
  /// In zh, this message translates to:
  /// **'连接方式'**
  String get deviceConnectMode;

  /// No description provided for @deviceFixedHint.
  ///
  /// In zh, this message translates to:
  /// **'只连下面这个地址，不在局域网里找'**
  String get deviceFixedHint;

  /// No description provided for @deviceAutoHint.
  ///
  /// In zh, this message translates to:
  /// **'在局域网里找控制端，找不到时用上次连上的地址'**
  String get deviceAutoHint;

  /// No description provided for @deviceAuto.
  ///
  /// In zh, this message translates to:
  /// **'自动寻找'**
  String get deviceAuto;

  /// No description provided for @deviceFixed.
  ///
  /// In zh, this message translates to:
  /// **'固定地址'**
  String get deviceFixed;

  /// No description provided for @deviceServerAddress.
  ///
  /// In zh, this message translates to:
  /// **'控制端的地址'**
  String get deviceServerAddress;

  /// No description provided for @deviceAddressExample.
  ///
  /// In zh, this message translates to:
  /// **'例如 192.168.2.12 或 192.168.2.12:24810'**
  String get deviceAddressExample;

  /// No description provided for @deviceRestoreDefaults.
  ///
  /// In zh, this message translates to:
  /// **'恢复默认设置'**
  String get deviceRestoreDefaults;

  /// No description provided for @deviceRestoreDefaultsHint.
  ///
  /// In zh, this message translates to:
  /// **'鼠标、按键、切换与共享都回到默认，名字保留'**
  String get deviceRestoreDefaultsHint;

  /// No description provided for @deviceRestore.
  ///
  /// In zh, this message translates to:
  /// **'恢复'**
  String get deviceRestore;

  /// No description provided for @deviceRemove.
  ///
  /// In zh, this message translates to:
  /// **'移除这台设备'**
  String get deviceRemove;

  /// No description provided for @deviceRemoveConfirm.
  ///
  /// In zh, this message translates to:
  /// **'确定吗？移除后需要重新配对，这台的设置一并删除'**
  String get deviceRemoveConfirm;

  /// No description provided for @deviceRemoveHint.
  ///
  /// In zh, this message translates to:
  /// **'移除后需要重新配对'**
  String get deviceRemoveHint;

  /// No description provided for @deviceRemoveButton.
  ///
  /// In zh, this message translates to:
  /// **'移除'**
  String get deviceRemoveButton;

  /// No description provided for @phStarted.
  ///
  /// In zh, this message translates to:
  /// **'ScreenFerry 已启动'**
  String get phStarted;

  /// No description provided for @phStartedServer.
  ///
  /// In zh, this message translates to:
  /// **'{name} · 键鼠在本机'**
  String phStartedServer(String name);

  /// No description provided for @phStartedClient.
  ///
  /// In zh, this message translates to:
  /// **'{name} · 由另一台电脑控制'**
  String phStartedClient(String name);

  /// No description provided for @phCannotShare.
  ///
  /// In zh, this message translates to:
  /// **'无法开始共享'**
  String get phCannotShare;

  /// No description provided for @phListening.
  ///
  /// In zh, this message translates to:
  /// **'已开始共享'**
  String get phListening;

  /// No description provided for @phListeningAnnounced.
  ///
  /// In zh, this message translates to:
  /// **'等待另一台电脑连接 · 端口 {port}'**
  String phListeningAnnounced(String port);

  /// No description provided for @phListeningQuiet.
  ///
  /// In zh, this message translates to:
  /// **'端口 {port} · 没有在局域网里广播'**
  String phListeningQuiet(String port);

  /// No description provided for @phConnected.
  ///
  /// In zh, this message translates to:
  /// **'已连接 {name}'**
  String phConnected(String name);

  /// No description provided for @phDisconnected.
  ///
  /// In zh, this message translates to:
  /// **'与 {name} 断开'**
  String phDisconnected(String name);

  /// No description provided for @phPeerClosed.
  ///
  /// In zh, this message translates to:
  /// **'对方已断开'**
  String get phPeerClosed;

  /// No description provided for @phPeerOpenedSettings.
  ///
  /// In zh, this message translates to:
  /// **'{name} 请本机打开设置'**
  String phPeerOpenedSettings(String name);

  /// No description provided for @phShowingItsSettings.
  ///
  /// In zh, this message translates to:
  /// **'已翻到它的设置页'**
  String get phShowingItsSettings;

  /// No description provided for @phCursorToPeer.
  ///
  /// In zh, this message translates to:
  /// **'光标切到 {name}'**
  String phCursorToPeer(String name);

  /// No description provided for @phAcrossEdge.
  ///
  /// In zh, this message translates to:
  /// **'从本机{side, select, left{左侧} right{右侧} top{上方} other{下方}}边缘'**
  String phAcrossEdge(String side);

  /// No description provided for @phCursorBack.
  ///
  /// In zh, this message translates to:
  /// **'光标回到本机'**
  String get phCursorBack;

  /// No description provided for @phShortcut.
  ///
  /// In zh, this message translates to:
  /// **'快捷键 {keys}'**
  String phShortcut(String keys);

  /// No description provided for @phFromPeerEdge.
  ///
  /// In zh, this message translates to:
  /// **'从对方屏幕边缘'**
  String get phFromPeerEdge;

  /// No description provided for @phCursorHere.
  ///
  /// In zh, this message translates to:
  /// **'光标来到本机'**
  String get phCursorHere;

  /// No description provided for @phFrom.
  ///
  /// In zh, this message translates to:
  /// **'从 {name}'**
  String phFrom(String name);

  /// No description provided for @phCursorReturned.
  ///
  /// In zh, this message translates to:
  /// **'光标回到 {name}'**
  String phCursorReturned(String name);

  /// No description provided for @phClipToPeer.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板同步到 {name}'**
  String phClipToPeer(String name);

  /// No description provided for @phClipHere.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板同步到本机'**
  String get phClipHere;

  /// No description provided for @phClipTooBigToPeer.
  ///
  /// In zh, this message translates to:
  /// **'剪贴板里的{format, select, image{图片} text{文本} other{带格式的内容}}太大，没有同步到 {name}'**
  String phClipTooBigToPeer(String format, String name);

  /// No description provided for @phClipTooBigHere.
  ///
  /// In zh, this message translates to:
  /// **'{name} 剪贴板里的{format, select, image{图片} text{文本} other{带格式的内容}}太大，没有同步过来'**
  String phClipTooBigHere(String format, String name);

  /// No description provided for @phClipLimit.
  ///
  /// In zh, this message translates to:
  /// **'{size}，超过了 {limit} 的上限 · 可以在设置里调大'**
  String phClipLimit(String size, String limit);

  /// No description provided for @phPaired.
  ///
  /// In zh, this message translates to:
  /// **'已与 {name} 配对'**
  String phPaired(String name);

  /// No description provided for @phRejectedByPeer.
  ///
  /// In zh, this message translates to:
  /// **'{name} 拒绝了配对'**
  String phRejectedByPeer(String name);

  /// No description provided for @phRejected.
  ///
  /// In zh, this message translates to:
  /// **'已拒绝 {name} 的配对请求'**
  String phRejected(String name);

  /// No description provided for @phPairingRequest.
  ///
  /// In zh, this message translates to:
  /// **'{name} 请求配对'**
  String phPairingRequest(String name);

  /// No description provided for @phPairingCode.
  ///
  /// In zh, this message translates to:
  /// **'配对码 {code}'**
  String phPairingCode(String code);

  /// No description provided for @phItems.
  ///
  /// In zh, this message translates to:
  /// **'{file} 等 {items} 项'**
  String phItems(String file, int items, int more);

  /// No description provided for @phFiles.
  ///
  /// In zh, this message translates to:
  /// **'{files} 个文件 · {size}'**
  String phFiles(int files, String size);

  /// No description provided for @phSending.
  ///
  /// In zh, this message translates to:
  /// **'发送 {what} 到 {name}'**
  String phSending(String name, String what);

  /// No description provided for @phReceiving.
  ///
  /// In zh, this message translates to:
  /// **'{name} 发来 {what}'**
  String phReceiving(String name, String what);

  /// No description provided for @phSent.
  ///
  /// In zh, this message translates to:
  /// **'已发送 {what}'**
  String phSent(String what);

  /// No description provided for @phSentTo.
  ///
  /// In zh, this message translates to:
  /// **'到 {name} · {size}'**
  String phSentTo(String name, String size);

  /// No description provided for @phReceived.
  ///
  /// In zh, this message translates to:
  /// **'已收到 {what}'**
  String phReceived(String what);

  /// No description provided for @phSavedIn.
  ///
  /// In zh, this message translates to:
  /// **'保存在 {folder}'**
  String phSavedIn(String folder);

  /// No description provided for @phInterrupted.
  ///
  /// In zh, this message translates to:
  /// **'传输中断：{what}'**
  String phInterrupted(String what);

  /// No description provided for @phResumesLater.
  ///
  /// In zh, this message translates to:
  /// **'重新连上后接着传'**
  String get phResumesLater;

  /// No description provided for @phCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消：{what}'**
  String phCancelled(String what);

  /// No description provided for @phCancelledByPeer.
  ///
  /// In zh, this message translates to:
  /// **'对方取消了'**
  String get phCancelledByPeer;

  /// No description provided for @phCancelledHere.
  ///
  /// In zh, this message translates to:
  /// **'本机取消了'**
  String get phCancelledHere;

  /// No description provided for @phFailed.
  ///
  /// In zh, this message translates to:
  /// **'传输失败：{what}'**
  String phFailed(String what);

  /// No description provided for @phSharingResumed.
  ///
  /// In zh, this message translates to:
  /// **'继续共享'**
  String get phSharingResumed;

  /// No description provided for @phSharingResumedHint.
  ///
  /// In zh, this message translates to:
  /// **'键鼠可以切到对方'**
  String get phSharingResumedHint;

  /// No description provided for @phSharingPaused.
  ///
  /// In zh, this message translates to:
  /// **'暂停共享'**
  String get phSharingPaused;

  /// No description provided for @phSharingPausedHint.
  ///
  /// In zh, this message translates to:
  /// **'键鼠留在本机'**
  String get phSharingPausedHint;

  /// No description provided for @phArranged.
  ///
  /// In zh, this message translates to:
  /// **'调整了设备的位置'**
  String get phArranged;

  /// No description provided for @phLayoutSaved.
  ///
  /// In zh, this message translates to:
  /// **'布局已保存'**
  String get phLayoutSaved;

  /// No description provided for @phOtherDevices.
  ///
  /// In zh, this message translates to:
  /// **'其他设备'**
  String get phOtherDevices;

  /// No description provided for @phMoved.
  ///
  /// In zh, this message translates to:
  /// **'{name} 移到{where}'**
  String phMoved(String name, String where);

  /// No description provided for @phNowServer.
  ///
  /// In zh, this message translates to:
  /// **'改为控制别的电脑'**
  String get phNowServer;

  /// No description provided for @phNowServerHint.
  ///
  /// In zh, this message translates to:
  /// **'键盘鼠标在本机，等另一台电脑来连接'**
  String get phNowServerHint;

  /// No description provided for @phNowClient.
  ///
  /// In zh, this message translates to:
  /// **'改为由另一台电脑控制'**
  String get phNowClient;

  /// No description provided for @phNowClientHint.
  ///
  /// In zh, this message translates to:
  /// **'在局域网里寻找键盘鼠标所在的那台'**
  String get phNowClientHint;

  /// No description provided for @phRemoved.
  ///
  /// In zh, this message translates to:
  /// **'已移除 {name}'**
  String phRemoved(String name);

  /// No description provided for @phPairAgain.
  ///
  /// In zh, this message translates to:
  /// **'下次连接需要重新配对'**
  String get phPairAgain;

  /// No description provided for @toastPeerSettingsOpened.
  ///
  /// In zh, this message translates to:
  /// **'已在 {name} 上打开设置，把光标移过去就能改'**
  String toastPeerSettingsOpened(String name);

  /// No description provided for @toastPeerSettingsUnsupported.
  ///
  /// In zh, this message translates to:
  /// **'{name} 上的 ScreenFerry 版本较旧，请在那台电脑上打开设置'**
  String toastPeerSettingsUnsupported(String name);

  /// No description provided for @toastDiagnosticsMissing.
  ///
  /// In zh, this message translates to:
  /// **'诊断文件已经不在了，请重新导出'**
  String get toastDiagnosticsMissing;

  /// No description provided for @toastNotConnectedPeer.
  ///
  /// In zh, this message translates to:
  /// **'还没有连上对方，或者对方不支持传文件'**
  String get toastNotConnectedPeer;

  /// No description provided for @toastFilesDisabled.
  ///
  /// In zh, this message translates to:
  /// **'已在设置里关掉了拖文件给 {name}'**
  String toastFilesDisabled(String name);

  /// No description provided for @toastSending.
  ///
  /// In zh, this message translates to:
  /// **'开始发给 {name}'**
  String toastSending(String name);

  /// No description provided for @toastSharingResumed.
  ///
  /// In zh, this message translates to:
  /// **'已继续共享'**
  String get toastSharingResumed;

  /// No description provided for @toastSharingPaused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停共享'**
  String get toastSharingPaused;

  /// No description provided for @toastAutostartFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法修改开机自启：{message}'**
  String toastAutostartFailed(String message);

  /// No description provided for @toastSaveFailed.
  ///
  /// In zh, this message translates to:
  /// **'保存设置失败：{message}'**
  String toastSaveFailed(String message);

  /// No description provided for @toastPeerOffline.
  ///
  /// In zh, this message translates to:
  /// **'{name} 现在没连着'**
  String toastPeerOffline(String name);

  /// No description provided for @toastPeersReadFailed.
  ///
  /// In zh, this message translates to:
  /// **'读取已配对设备失败：{message}'**
  String toastPeersReadFailed(String message);

  /// No description provided for @toastRemoveFailed.
  ///
  /// In zh, this message translates to:
  /// **'移除失败：{message}'**
  String toastRemoveFailed(String message);

  /// No description provided for @toastServerGone.
  ///
  /// In zh, this message translates to:
  /// **'这台电脑已经不在局域网里了'**
  String get toastServerGone;

  /// No description provided for @toastPausedConnectLater.
  ///
  /// In zh, this message translates to:
  /// **'已暂停共享，继续共享后再连接'**
  String get toastPausedConnectLater;

  /// No description provided for @toastNotSharingInviteLater.
  ///
  /// In zh, this message translates to:
  /// **'还没开始共享，开始后再邀请'**
  String get toastNotSharingInviteLater;

  /// No description provided for @toastDiagnosticsFailed.
  ///
  /// In zh, this message translates to:
  /// **'导出诊断文件失败：{message}'**
  String toastDiagnosticsFailed(String message);

  /// No description provided for @layoutDropAtNth.
  ///
  /// In zh, this message translates to:
  /// **'松手后放到本机{side, select, left{左侧，从上往下} right{右侧，从上往下} top{上方，从左往右} other{下方，从左往右}}第 {n} 台'**
  String layoutDropAtNth(String side, int n);

  /// No description provided for @aboutVersionLine.
  ///
  /// In zh, this message translates to:
  /// **'版本 {version} · 在多台电脑之间共享一套键盘鼠标'**
  String aboutVersionLine(String version);

  /// No description provided for @updateCurrent.
  ///
  /// In zh, this message translates to:
  /// **'当前版本 {version}'**
  String updateCurrent(String version);

  /// No description provided for @updateUpToDate.
  ///
  /// In zh, this message translates to:
  /// **'已是最新版本'**
  String get updateUpToDate;

  /// No description provided for @updateCheckedAgo.
  ///
  /// In zh, this message translates to:
  /// **'{version} · {ago}检查'**
  String updateCheckedAgo(String version, String ago);

  /// No description provided for @updateChecking.
  ///
  /// In zh, this message translates to:
  /// **'正在检查更新…'**
  String get updateChecking;

  /// No description provided for @updateCheckingHint.
  ///
  /// In zh, this message translates to:
  /// **'只取一份很小的更新信息，不发送任何设备信息'**
  String get updateCheckingHint;

  /// No description provided for @updateFound.
  ///
  /// In zh, this message translates to:
  /// **'发现新版本 {version}'**
  String updateFound(String version);

  /// No description provided for @updateReleased.
  ///
  /// In zh, this message translates to:
  /// **'{date}发布'**
  String updateReleased(String date);

  /// No description provided for @updateDownloadedHint.
  ///
  /// In zh, this message translates to:
  /// **'已在后台下载好'**
  String get updateDownloadedHint;

  /// No description provided for @updateDownload.
  ///
  /// In zh, this message translates to:
  /// **'下载'**
  String get updateDownload;

  /// No description provided for @updateDownloading.
  ///
  /// In zh, this message translates to:
  /// **'正在下载 {version}'**
  String updateDownloading(String version);

  /// No description provided for @updateRestartHint.
  ///
  /// In zh, this message translates to:
  /// **'重启时键鼠会断开约 5 秒'**
  String get updateRestartHint;

  /// No description provided for @updateLater.
  ///
  /// In zh, this message translates to:
  /// **'稍后'**
  String get updateLater;

  /// No description provided for @updateRestart.
  ///
  /// In zh, this message translates to:
  /// **'重启并更新'**
  String get updateRestart;

  /// No description provided for @updateInstall.
  ///
  /// In zh, this message translates to:
  /// **'安装'**
  String get updateInstall;

  /// No description provided for @updateLocalConfirm.
  ///
  /// In zh, this message translates to:
  /// **'安装时 Windows 会弹出管理员确认框，从另一台电脑操作时点不到它。请用这台电脑自己的键盘鼠标点“安装”'**
  String get updateLocalConfirm;

  /// No description provided for @updateInstalling.
  ///
  /// In zh, this message translates to:
  /// **'正在安装 {version}…'**
  String updateInstalling(String version);

  /// No description provided for @updateInstallingHint.
  ///
  /// In zh, this message translates to:
  /// **'ScreenFerry 会自己重新打开，这个窗口随后重新连上'**
  String get updateInstallingHint;

  /// No description provided for @updateManual.
  ///
  /// In zh, this message translates to:
  /// **'发现新版本 {version}，要手动安装'**
  String updateManual(String version);

  /// No description provided for @updateManualHint.
  ///
  /// In zh, this message translates to:
  /// **'这份不能自己更新：可能是自己打包的开发版本，或者放在没有写权限的位置'**
  String get updateManualHint;

  /// No description provided for @updateOpenDownloads.
  ///
  /// In zh, this message translates to:
  /// **'打开下载页面'**
  String get updateOpenDownloads;

  /// No description provided for @updateUnreachable.
  ///
  /// In zh, this message translates to:
  /// **'连不上更新服务器'**
  String get updateUnreachable;

  /// No description provided for @updateUnreachableHint.
  ///
  /// In zh, this message translates to:
  /// **'已按系统的代理设置连接，也可以在浏览器里下载'**
  String get updateUnreachableHint;

  /// No description provided for @updateBrowserDownload.
  ///
  /// In zh, this message translates to:
  /// **'浏览器下载'**
  String get updateBrowserDownload;

  /// No description provided for @updateRetry.
  ///
  /// In zh, this message translates to:
  /// **'重试'**
  String get updateRetry;

  /// No description provided for @updateVerifyFailed.
  ///
  /// In zh, this message translates to:
  /// **'安装包没有通过校验，已删除'**
  String get updateVerifyFailed;

  /// No description provided for @updateVerifyFailedHint.
  ///
  /// In zh, this message translates to:
  /// **'什么都没有安装。可能是网络里有东西改了文件，换个网络再试'**
  String get updateVerifyFailedHint;

  /// No description provided for @updateStale.
  ///
  /// In zh, this message translates to:
  /// **'无法确认是不是最新版'**
  String get updateStale;

  /// No description provided for @updateStaleHint.
  ///
  /// In zh, this message translates to:
  /// **'拿到的更新信息已经过期，可能有人挡住了新版本'**
  String get updateStaleHint;

  /// No description provided for @updateNoKeys.
  ///
  /// In zh, this message translates to:
  /// **'这份程序不能检查更新'**
  String get updateNoKeys;

  /// No description provided for @updateNoKeysHint.
  ///
  /// In zh, this message translates to:
  /// **'它没有内置发布公钥，通常是自己打包的开发版本'**
  String get updateNoKeysHint;

  /// No description provided for @updateInstallFailed.
  ///
  /// In zh, this message translates to:
  /// **'更新没有装上'**
  String get updateInstallFailed;

  /// No description provided for @updateBreaksPeers.
  ///
  /// In zh, this message translates to:
  /// **'两台要一起更新'**
  String get updateBreaksPeers;

  /// No description provided for @updateBreaksPeersHint.
  ///
  /// In zh, this message translates to:
  /// **'{version} 改了两台之间的通信方式：另一台也更新到 {version} 之后才能再连上'**
  String updateBreaksPeersHint(String version);

  /// No description provided for @updateAuto.
  ///
  /// In zh, this message translates to:
  /// **'自动检查更新'**
  String get updateAuto;

  /// No description provided for @updateAutoHint.
  ///
  /// In zh, this message translates to:
  /// **'每天检查一次，有新版本时在后台下载好，由你决定何时重启'**
  String get updateAutoHint;

  /// No description provided for @updateLastChecked.
  ///
  /// In zh, this message translates to:
  /// **'上次检查：{ago}'**
  String updateLastChecked(String ago);

  /// No description provided for @updateSidebar.
  ///
  /// In zh, this message translates to:
  /// **'更新到 {version}'**
  String updateSidebar(String version);

  /// No description provided for @updateBannerTitle.
  ///
  /// In zh, this message translates to:
  /// **'新版本 {version} 已准备好'**
  String updateBannerTitle(String version);

  /// No description provided for @updateBannerHint.
  ///
  /// In zh, this message translates to:
  /// **'重启 ScreenFerry 完成更新，键鼠会断开约 5 秒'**
  String get updateBannerHint;

  /// No description provided for @devicePeerCanUpdate.
  ///
  /// In zh, this message translates to:
  /// **'可以更新到 {version}'**
  String devicePeerCanUpdate(String version);

  /// No description provided for @phUpdateInstalled.
  ///
  /// In zh, this message translates to:
  /// **'已更新到 {to}'**
  String phUpdateInstalled(String to);

  /// No description provided for @phFromVersion.
  ///
  /// In zh, this message translates to:
  /// **'从 {from}'**
  String phFromVersion(String from);

  /// No description provided for @phUpdateNotInstalled.
  ///
  /// In zh, this message translates to:
  /// **'{version} 没有装上'**
  String phUpdateNotInstalled(String version);

  /// No description provided for @phStillOn.
  ///
  /// In zh, this message translates to:
  /// **'还在用 {current}'**
  String phStillOn(String current);

  /// No description provided for @toastUpdateCancelled.
  ///
  /// In zh, this message translates to:
  /// **'没有得到管理员确认，没有安装'**
  String get toastUpdateCancelled;

  /// No description provided for @toastUpdateFailed.
  ///
  /// In zh, this message translates to:
  /// **'更新没有装上：{message}'**
  String toastUpdateFailed(String message);
}

class _AppLocalizationsDelegate extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>['en', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
