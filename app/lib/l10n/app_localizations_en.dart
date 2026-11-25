// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'ScreenFerry';

  @override
  String get commonSave => 'Save';

  @override
  String get commonCancel => 'Cancel';

  @override
  String get commonClose => 'Close';

  @override
  String get navOverview => 'Dashboard';

  @override
  String get navDevices => 'Devices';

  @override
  String get navLayout => 'Layout';

  @override
  String get navLog => 'Log';

  @override
  String get navSettings => 'Settings';

  @override
  String get sidebarExpand => 'Expand sidebar';

  @override
  String get sidebarCollapse => 'Collapse sidebar';

  @override
  String get statusConnectingDaemon => 'Connecting to the background service';

  @override
  String get statusDaemonNotRunning => 'Background service not running';

  @override
  String get statusDaemonLost => 'Disconnected from the background service';

  @override
  String get statusSharing => 'Sharing';

  @override
  String statusSharingRtt(String ms) {
    return 'Sharing · $ms ms';
  }

  @override
  String get statusPaused => 'Sharing paused';

  @override
  String get statusWaiting => 'Waiting for a connection';

  @override
  String get statusSearching => 'Looking for the other computer';

  @override
  String get statusConnecting => 'Connecting';

  @override
  String get statusStarting => 'Starting';

  @override
  String get statusStopped => 'Stopped';

  @override
  String get windowMinimize => 'Minimize';

  @override
  String get windowRestore => 'Restore';

  @override
  String get windowMaximize => 'Maximize';

  @override
  String get chartSwitchesLabel => 'Switches per hour today';

  @override
  String get chartMidnight => '0:00';

  @override
  String get chartNoon => '12:00';

  @override
  String get chartNow => 'Now';

  @override
  String get bannerDaemonLost => 'Lost the connection to the background service. Reconnecting…';

  @override
  String get bannerPermission => 'Keyboard and mouse access needed';

  @override
  String get bannerPortInUse => 'Port in use';

  @override
  String get bannerOther => 'The background service ran into a problem';

  @override
  String get bannerOpenSystemSettings => 'Open System Settings';

  @override
  String get problemPermission =>
      'Allow ScreenFerry to control the keyboard and mouse in System Settings → Privacy & Security';

  @override
  String problemPortInUse(int port) {
    return 'Port $port is in use: another keyboard and mouse sharing app or another copy of ScreenFerry may be running';
  }

  @override
  String get offlineConnecting => 'Connecting to the background service…';

  @override
  String get offlineNotRunning => 'The background service isn’t running';

  @override
  String get offlineStartHint => 'Start ScreenFerry first. This page fills in once it’s connected.';

  @override
  String get offlineAutoConnect => 'It connects automatically once started.';

  @override
  String get offlineStartFailed => 'Couldn’t start it. Open ScreenFerry from your Applications folder.';

  @override
  String get offlineStarting => 'Starting…';

  @override
  String get offlineStart => 'Start ScreenFerry';

  @override
  String get offlineRetrying => 'Retrying';

  @override
  String get nameEditorRename => 'Rename';

  @override
  String get accentBlue => 'Blue';

  @override
  String get accentTeal => 'Teal';

  @override
  String get accentPurple => 'Purple';

  @override
  String get accentOrange => 'Orange';

  @override
  String get accentPink => 'Pink';

  @override
  String get aboutTitle => 'About';

  @override
  String get aboutBack => 'Back to Settings';

  @override
  String get aboutTagline => 'Share one keyboard and mouse across your computers';

  @override
  String get aboutCheckUpdates => 'Check for Updates';

  @override
  String get aboutUpdatesSoon => 'Automatic update checks are coming soon';

  @override
  String get aboutAuthor => 'Author';

  @override
  String get aboutLicense => 'License';

  @override
  String get logTitle => 'Log';

  @override
  String get logClear => 'Clear';

  @override
  String get logFilter => 'Filter';

  @override
  String get logAll => 'All';

  @override
  String get logCursor => 'Cursor';

  @override
  String get logClipboard => 'Clipboard';

  @override
  String get logFiles => 'Files';

  @override
  String get logLinks => 'Devices';

  @override
  String get logRun => 'Diagnostics';

  @override
  String get logEmpty => 'Nothing yet';

  @override
  String logCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(count, locale: localeName, other: '$count entries', one: '1 entry');
    return '$_temp0';
  }

  @override
  String logRunSize(String size, String lines) {
    return '$size · $lines lines';
  }

  @override
  String get pairingTitle => 'Pairing';

  @override
  String pairingWith(String name) {
    return 'Pair with “$name”';
  }

  @override
  String get pairingCheck => 'Check that both computers show the same code. Only confirm if they match.';

  @override
  String get pairingPeerFingerprint => 'Their fingerprint ';

  @override
  String get pairingWaiting => 'Confirmed. Waiting for the other computer…';

  @override
  String pairingAutoReject(int seconds) {
    return 'Declines automatically in $seconds s';
  }

  @override
  String get pairingTimedOut => 'Timed out';

  @override
  String get pairingReject => 'Decline';

  @override
  String get pairingAccept => 'They match, pair';

  @override
  String pairingCodeLabel(String code) {
    return 'Pairing code $code';
  }

  @override
  String pairingDone(String name) {
    return 'Paired with “$name”';
  }

  @override
  String get pairingNoMoreAsking => 'It won’t ask again next time';

  @override
  String dialBadAddressTitle(String address) {
    return 'Not a valid address: $address';
  }

  @override
  String get dialBadAddressBody => 'Enter an IP (e.g. 192.168.2.12), IP:port, or a computer name.';

  @override
  String dialNotWaitingInviteTitle(String address) {
    return '$address isn’t waiting for an invitation';
  }

  @override
  String get dialNotWaitingInviteBody =>
      'Make sure ScreenFerry is running there and set to “Be controlled”. Older versions can’t be invited and need an update first.';

  @override
  String dialNotWaitingTitle(String address) {
    return '$address isn’t waiting for a connection';
  }

  @override
  String get dialNotWaitingBody =>
      'Make sure ScreenFerry is running there and set to “Control other computers” in Settings.';

  @override
  String dialUnreachableTitle(String address) {
    return 'Can’t reach $address';
  }

  @override
  String get dialUnreachableBody =>
      'Make sure both computers are on the same network and the address is right. On Windows, allow ScreenFerry through the firewall when asked.';

  @override
  String dialNotScreenFerryTitle(String address) {
    return '$address isn’t running ScreenFerry';
  }

  @override
  String get dialNotScreenFerryBody => 'Check the address and port.';

  @override
  String get dialIncompatibleTitle => 'Incompatible versions';

  @override
  String get dialIncompatibleBody =>
      'The two computers run very different versions of ScreenFerry. Update both to the latest version.';

  @override
  String dialBusyTitle(String address) {
    return '$address is connected to another computer';
  }

  @override
  String get dialBusyBody => 'Disconnect it there first, then invite it again.';

  @override
  String get dialWrongRoleInviteTitle => 'This computer is the one being controlled';

  @override
  String get dialWrongRoleInviteBody =>
      'Inviting is for the controlling computer. Use “Connect by Address” here instead.';

  @override
  String get dialWrongRoleTitle => 'This computer is the controlling one';

  @override
  String get dialWrongRoleBody =>
      'Connecting by address is for the computer being controlled. Use “Invite a Computer” here instead.';

  @override
  String get dialInviteTitle => 'Invite a Computer';

  @override
  String get dialConnectTitle => 'Connect by Address';

  @override
  String dialInvited(String address) {
    return 'Invited $address. It’s connecting now';
  }

  @override
  String dialConnected(String address) {
    return 'Connected to $address';
  }

  @override
  String get dialInviteHint =>
      'ScreenFerry must be running there, set to “Be controlled”. Its address is on its Devices page. Port 24811 is used if you leave it out.';

  @override
  String get dialConnectHint =>
      'An IP, IP:port or computer name. Port 24810 is used if you leave it out. The address is on that computer’s Devices page.';

  @override
  String get dialInviteSubtitle => 'Enter the address of the computer to control. It connects back automatically';

  @override
  String get dialConnectSubtitle =>
      'Enter the address of the controlling computer (the one with the keyboard and mouse)';

  @override
  String get dialAddress => 'Address';

  @override
  String dialExample(String address) {
    return 'e.g. $address';
  }

  @override
  String dialInviting(String address) {
    return 'Inviting $address…';
  }

  @override
  String dialConnecting(String address) {
    return 'Connecting to $address…';
  }

  @override
  String get dialRetry => 'Try Again';

  @override
  String get dialInvite => 'Invite';

  @override
  String get dialConnect => 'Connect';

  @override
  String get diagTitle => 'Attach a diagnostics file when reporting a problem';

  @override
  String get diagBody =>
      'Includes the version, system, settings and recent diagnostics log. Device names, user names and file names are replaced with placeholders. No clipboard contents or keys.';

  @override
  String get diagCopySummary => 'Copy Summary';

  @override
  String get diagExport => 'Export Diagnostics';

  @override
  String get diagOpenFolder => 'Open Log Folder';

  @override
  String get diagReport => 'Report on GitHub';

  @override
  String get diagSaved => 'Saved to Downloads: ';

  @override
  String get diagShowInFinder => 'Show in Finder';

  @override
  String get diagShowInExplorer => 'Show in File Explorer';

  @override
  String get diagVerbose => 'Detailed Logging';

  @override
  String get diagVerboseOff =>
      'Turn on while troubleshooting connections to log more detail. Turns off when ScreenFerry restarts';

  @override
  String get diagVerboseJustOn => 'Just turned on. Export after the problem happens, then turn it off';

  @override
  String diagVerboseOnFor(int minutes) {
    String _temp0 = intl.Intl.pluralLogic(
      minutes,
      locale: localeName,
      other: 'On for $minutes minutes',
      one: 'On for 1 minute',
    );
    return '$_temp0. Export after the problem happens, then turn it off';
  }

  @override
  String get diagRecent => 'Recent Log';

  @override
  String get diagLive => 'Live';

  @override
  String get diagNotConnected => 'Not connected';

  @override
  String get diagLevel => 'Log level';

  @override
  String get diagProblemsOnly => 'Warnings & Errors';

  @override
  String get diagNoProblems => 'No recent warnings or errors';

  @override
  String get diagNoLog => 'No log yet';

  @override
  String get diagLevelDebug => 'Debug';

  @override
  String get diagLevelInfo => 'Info';

  @override
  String get diagLevelWarn => 'Warn';

  @override
  String get diagLevelError => 'Error';

  @override
  String get diagCopied =>
      'Summary copied: version, system and the last 200 lines. Paste it into the issue description';

  @override
  String get keyMapTitle => 'Add Key Mapping';

  @override
  String get keyMapNothing => 'Nothing';

  @override
  String keyMapOnlyFor(String name) {
    return 'Applies to $name only';
  }

  @override
  String get keyMapStep1 => '① Key pressed here';

  @override
  String get keyMapRecording => 'Recording, press a key';

  @override
  String get keyMapRerecord => 'Record Again';

  @override
  String get keyMapPressAKey => 'Press a key on the keyboard…';

  @override
  String get keyMapEscCancels => 'Esc to cancel';

  @override
  String get keyMapRecorded => 'Recorded';

  @override
  String get keyMapClickToRecord => 'Click here, then press a key';

  @override
  String get keyMapModifiersToo => 'Single modifier keys work too, left and right count separately.';

  @override
  String keyMapStep2(String name) {
    return '② What $name receives';
  }

  @override
  String get keyMapSearch => 'Search keys, e.g. Home, F5, volume';

  @override
  String get keyMapPress => 'Press';

  @override
  String keyMapGetsNothing(String name) {
    return '$name receives nothing';
  }

  @override
  String keyMapGets(String name) {
    return '$name receives';
  }

  @override
  String get keyMapAdd => 'Add';

  @override
  String get sideLeft => 'Left';

  @override
  String get sideRight => 'Right';

  @override
  String get sideTop => 'Top';

  @override
  String get sideBottom => 'Bottom';

  @override
  String get peerFallback => 'the other computer';

  @override
  String get toastLinkCopied => 'Couldn’t open the browser. Link copied';

  @override
  String get toastNotConnected => 'Not connected to the background service yet. Try again shortly';

  @override
  String toastConnectingTo(String name) {
    return 'Connecting to $name…';
  }

  @override
  String get timeJustNow => 'just now';

  @override
  String timeSecondsAgo(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: '$n seconds ago', one: '1 second ago');
    return '$_temp0';
  }

  @override
  String timeMinutesAgo(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: '$n minutes ago', one: '1 minute ago');
    return '$_temp0';
  }

  @override
  String timeHoursAgo(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: '$n hours ago', one: '1 hour ago');
    return '$_temp0';
  }

  @override
  String timeDaysAgo(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: '$n days ago', one: '1 day ago');
    return '$_temp0';
  }

  @override
  String get seenOnline => 'Online';

  @override
  String get seenOffline => 'Offline';

  @override
  String seenAgo(String ago) {
    return 'Last seen $ago';
  }

  @override
  String get durationUnderMinute => 'Under a minute';

  @override
  String durationMinutes(int m) {
    String _temp0 = intl.Intl.pluralLogic(m, locale: localeName, other: '$m minutes', one: '1 minute');
    return '$_temp0';
  }

  @override
  String durationHours(int h, int m) {
    return '$h h $m min';
  }

  @override
  String keyMedia(String hex) {
    return 'Media key 0x$hex';
  }

  @override
  String keyUnknown(String hex) {
    return 'Key 0x$hex';
  }

  @override
  String keyNumpad(String key) {
    return 'Numpad $key';
  }

  @override
  String keyLeft(String key) {
    return 'Left $key';
  }

  @override
  String keyRight(String key) {
    return 'Right $key';
  }

  @override
  String get keySpace => 'Space';

  @override
  String get keyIso => '\\ (ISO)';

  @override
  String get keyMenu => 'Menu';

  @override
  String get keyMute => 'Mute';

  @override
  String get keyVolumeUp => 'Volume Up';

  @override
  String get keyVolumeDown => 'Volume Down';

  @override
  String get keyHangul => 'Hangul/English, Kana';

  @override
  String get keyHanja => 'Hanja, Eisu';

  @override
  String get keyForwardDelete => '⌦ Forward Delete';

  @override
  String get keyBrightnessUp => 'Brightness Up';

  @override
  String get keyBrightnessDown => 'Brightness Down';

  @override
  String get keyNextTrack => 'Next Track';

  @override
  String get keyPreviousTrack => 'Previous Track';

  @override
  String get keyStop => 'Stop';

  @override
  String get keyPlayPause => 'Play/Pause';

  @override
  String get settingsThisComputer => 'This Computer';

  @override
  String get settingsRole => 'Role';

  @override
  String get settingsRoleServerHint => 'The keyboard and mouse are attached here and control other computers';

  @override
  String get settingsRoleClientHint =>
      'Controlled by another computer’s keyboard and mouse · Reconnects after switching';

  @override
  String get settingsRoleServer => 'Control Others';

  @override
  String get settingsRoleClient => 'Be Controlled';

  @override
  String get settingsAppearance => 'Appearance';

  @override
  String get settingsTheme => 'Theme';

  @override
  String get settingsFollowSystem => 'System';

  @override
  String get settingsLight => 'Light';

  @override
  String get settingsDark => 'Dark';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get settingsAccent => 'Accent Color';

  @override
  String get settingsAccentHint => 'Used for switches, selections and buttons';

  @override
  String get settingsMouseClipboard => 'Mouse & Clipboard';

  @override
  String get settingsDpi => 'Mouse DPI on This Computer';

  @override
  String get settingsDpiHint => 'Cursor speed on every device is based on it';

  @override
  String get settingsPerDevice => 'Key Mapping, Scroll Direction';

  @override
  String get settingsPerDeviceHint => 'Set per device in Devices';

  @override
  String get settingsClipboard => 'Sync Clipboard';

  @override
  String get settingsClipboardHint => 'The other computer fetches the latest contents when the cursor moves over';

  @override
  String get settingsClipLimit => 'Size Limit per Item';

  @override
  String get settingsClipLimitHint => 'Images, text and other items larger than this aren’t synced';

  @override
  String get settingsGeneral => 'General';

  @override
  String get settingsAutostart => 'Start at Login';

  @override
  String get settingsAnnounce => 'Announce on the Local Network';

  @override
  String get settingsAnnounceHint => 'Other computers can find this one without entering an address';

  @override
  String get settingsDataDir => 'Data Folder';

  @override
  String get settingsAbout => 'About ScreenFerry';

  @override
  String settingsVersion(String version) {
    return 'Version $version';
  }

  @override
  String get commonComma => ', ';

  @override
  String get commonEnum => ', ';

  @override
  String get commonConnected => 'Connected';

  @override
  String get commonNotConnected => 'Not connected';

  @override
  String get commonThisComputer => 'This computer';

  @override
  String get layoutTitle => 'Layout';

  @override
  String layoutSetOnServer(String name) {
    return 'Screen positions are set on $name';
  }

  @override
  String get layoutSetOnServerHint =>
      'The keyboard and mouse are on that computer, so it decides which edge leads here.';

  @override
  String get layoutSwitchMode => 'Switching';

  @override
  String get layoutSwitchModeHint => 'Pausing at the edge before switching avoids accidental switches · Coming soon';

  @override
  String get layoutInstant => 'Instant';

  @override
  String get layoutDwell => 'Pause 0.15 s';

  @override
  String get layoutNoDragSwitch => 'Don’t Switch While Dragging';

  @override
  String get layoutNoDragSwitchHint => 'The cursor stays here while a mouse button is held · Always on for now';

  @override
  String get layoutBackHere => 'Back to This Computer';

  @override
  String get layoutBackHereHint => 'Press it to bring the cursor back if it gets stuck on the other screen';

  @override
  String get layoutGoPair => 'Pair a Device';

  @override
  String layoutWhere(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': 'left of this computer',
      'right': 'right of this computer',
      'top': 'above this computer',
      'other': 'below this computer',
    });
    return '$_temp0';
  }

  @override
  String layoutWhereNth(String side, int n) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': 'left of this computer, number $n from the top',
      'right': 'right of this computer, number $n from the top',
      'top': 'above this computer, number $n from the left',
      'other': 'below this computer, number $n from the left',
    });
    return '$_temp0';
  }

  @override
  String layoutPeerAt(String name, String where) {
    return '$name, $where';
  }

  @override
  String get layoutCursorOnIt => 'cursor is on it';

  @override
  String get layoutMovable => 'can be dragged';

  @override
  String get layoutEmpty =>
      'No paired devices yet. Run ScreenFerry on another computer and pair it, and it will appear next to this one';

  @override
  String layoutDropAt(String where) {
    return 'Release to place it $where';
  }

  @override
  String layoutSingle(String name, String where) {
    return '$name is $where. The cursor crosses the shared edge';
  }

  @override
  String layoutSideCount(String side, int n) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': 'on the left',
      'right': 'on the right',
      'top': 'above',
      'other': 'below',
    });
    return '$n $_temp0';
  }

  @override
  String layoutMany(String parts) {
    return 'Around this computer: $parts. Connected devices on the same side split that edge in order, and the cursor goes to whichever part it crosses';
  }

  @override
  String get layoutCursorHere => 'Cursor here';

  @override
  String get devicesThisName => 'Name of This Computer';

  @override
  String get devicesComputerName => 'Computer name';

  @override
  String get devicesNameHint => 'Other computers see this name. Leave it empty to use the computer name.';

  @override
  String get devicesThisFingerprint => 'This computer · Fingerprint ';

  @override
  String get devicesCopyFingerprint => 'Copy Fingerprint';

  @override
  String get devicesFingerprintCopied => 'Fingerprint copied';

  @override
  String get devicesPaired => 'Paired';

  @override
  String devicesCount(int n) {
    return '$n';
  }

  @override
  String get devicesNonePaired => 'No Paired Devices';

  @override
  String get devicesNonePairedHint =>
      'Run ScreenFerry on another computer. It appears here once both sides confirm the pairing code.';

  @override
  String get devicesFound => 'Nearby';

  @override
  String get devicesNotPairedYet => 'Not paired yet';

  @override
  String get devicesInvite => 'Invite a Computer';

  @override
  String get devicesConnectByAddress => 'Connect by Address';

  @override
  String get devicesWaiting => 'Waiting for Another Computer';

  @override
  String get devicesWaitingHint =>
      'Run ScreenFerry on another computer and choose this one. A pairing code will appear here for you to confirm.';

  @override
  String devicesWaitingHintAddress(String address) {
    return 'If it can’t find this computer on the network, connect by address $address there. If that computer has no keyboard and mouse, invite it from here.';
  }

  @override
  String get devicesNoneFound => 'No Other Devices Found';

  @override
  String get devicesNoneFoundHint =>
      'Some networks block automatic discovery (such as office networks or Wi‑Fi with device isolation). Connect by address instead. The address is on the Devices page of the controlling computer.';

  @override
  String get devicesThisAddress => 'Address';

  @override
  String get devicesAddressCopied => 'Address copied';

  @override
  String devicesCopy(String address) {
    return 'Copy $address';
  }

  @override
  String devicesCopyAddress(String address) {
    return 'Copy address $address';
  }

  @override
  String devicesOnlineAt(String address) {
    return 'Online · $address';
  }

  @override
  String devicesSettingsOf(String name) {
    return 'Settings for $name';
  }

  @override
  String get devicesFingerprint => 'Fingerprint ';

  @override
  String devicesPlace(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': 'Left of this computer',
      'right': 'Right of this computer',
      'top': 'Above this computer',
      'other': 'Below this computer',
    });
    return '$_temp0';
  }

  @override
  String get devicesSettings => 'Settings';

  @override
  String get devicesNotPaired => 'Not paired';

  @override
  String devicesFingerprintShort(String short) {
    return 'Fingerprint $short';
  }

  @override
  String get devicesAnnouncing => 'Announcing';

  @override
  String get devicesPair => 'Pair';

  @override
  String get overviewTitle => 'Dashboard';

  @override
  String get overviewSharing => 'Sharing';

  @override
  String get overviewPausedTitle => 'Sharing Paused';

  @override
  String get overviewPaused => 'Paused';

  @override
  String get overviewWaitingTitle => 'Waiting for a Connection';

  @override
  String get overviewWaiting => 'Waiting';

  @override
  String get overviewSearchingTitle => 'Looking for Devices';

  @override
  String get overviewSearching => 'Searching';

  @override
  String get overviewConnectingTitle => 'Connecting…';

  @override
  String get overviewConnecting => 'Connecting';

  @override
  String get overviewStartingTitle => 'Starting…';

  @override
  String get overviewStarting => 'Starting';

  @override
  String get overviewStoppedTitle => 'Can’t Share Right Now';

  @override
  String get overviewStopped => 'Stopped';

  @override
  String get overviewPausedHint => 'The keyboard and mouse stay here. Use the button on the right to resume';

  @override
  String get overviewAnnouncingHint =>
      'This computer is announced on the local network, so other computers will find it';

  @override
  String get overviewNotAnnouncingHint => 'This computer isn’t announced, so other computers need to enter its address';

  @override
  String get overviewSearchingHint => 'Looking for paired computers on the local network';

  @override
  String get overviewConnectingHint => 'Found it. Setting up an encrypted connection';

  @override
  String get overviewStartingHint => 'Just a moment';

  @override
  String get overviewProblemHint => 'The background service ran into a problem';

  @override
  String get overviewStatus => 'Status';

  @override
  String get overviewLatencyGreat => 'Latency excellent';

  @override
  String get overviewLatencyGood => 'Latency good';

  @override
  String get overviewLatencyHigh => 'Latency high';

  @override
  String overviewCursorOn(String name) {
    return 'Cursor on $name';
  }

  @override
  String get overviewCursorHere => 'Cursor here';

  @override
  String overviewConnectedCount(int n) {
    return '$n connected';
  }

  @override
  String get overviewAddress => 'Address';

  @override
  String get overviewEncryption => 'Encryption';

  @override
  String get overviewFingerprint => 'Fingerprint';

  @override
  String get overviewPause => 'Pause Sharing';

  @override
  String get overviewResume => 'Resume Sharing';

  @override
  String get overviewRtt => 'Round-Trip Time';

  @override
  String overviewPeak(String ms) {
    return 'Peak $ms';
  }

  @override
  String get overviewRttEmpty => 'Shown once connected';

  @override
  String overviewRttAverage(int n, String ms) {
    return 'Average over the last $n s: $ms ms';
  }

  @override
  String get overviewClipboard => 'Clipboard';

  @override
  String get overviewNoClip => 'Nothing Synced Yet';

  @override
  String get overviewNoClipHint => 'Copy something, move the cursor over, and paste on the other computer.';

  @override
  String get overviewClipOffHint => 'Clipboard sync is off. You can turn it on in Settings.';

  @override
  String overviewClipImage(int width, int height, String size) {
    return 'Image · $width×$height · $size';
  }

  @override
  String overviewClipRich(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: '$n characters', one: '1 character');
    return 'Formatted text · $_temp0';
  }

  @override
  String overviewClipText(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: '$n characters', one: '1 character');
    return 'Text · $_temp0';
  }

  @override
  String get overviewAnImage => 'An image';

  @override
  String get overviewNotText => '(Not text)';

  @override
  String overviewClipImageLabel(int width, int height) {
    return 'Clipboard image $width×$height';
  }

  @override
  String get overviewClipSync => 'Clipboard Sync';

  @override
  String get overviewAutostart => 'Start at Login';

  @override
  String get overviewAnnounce => 'Announce';

  @override
  String get overviewOn => 'On';

  @override
  String get overviewOff => 'Off';

  @override
  String get overviewManage => 'Manage';

  @override
  String overviewFoundNew(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n new devices found',
      one: '1 new device found',
    );
    return '$_temp0 on the local network';
  }

  @override
  String overviewThisName(String name) {
    return '$name · This computer';
  }

  @override
  String get overviewCopyFingerprint => 'Copy Fingerprint';

  @override
  String get overviewToday => 'Today';

  @override
  String overviewSwitches(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: 'screen switches', one: 'screen switch');
    return '$_temp0';
  }

  @override
  String overviewClips(int n) {
    String _temp0 = intl.Intl.pluralLogic(n, locale: localeName, other: 'clipboard syncs', one: 'clipboard sync');
    return '$_temp0';
  }

  @override
  String get overviewHours => 'hours connected';

  @override
  String get deviceLater => 'Applies when this computer controls it';

  @override
  String get deviceBack => 'Back to Devices';

  @override
  String get deviceMouse => 'Mouse';

  @override
  String get deviceInvertVertical => 'Reverse Vertical Scrolling';

  @override
  String get deviceInvertVerticalHint => 'Scroll up and down the other way';

  @override
  String get deviceInvertHorizontal => 'Reverse Horizontal Scrolling';

  @override
  String get deviceInvertHorizontalHint => 'Scroll left and right the other way';

  @override
  String get deviceScrollSpeed => 'Scroll Speed';

  @override
  String get deviceScrollSpeedHint => 'Adjust if small trackpad scrolls feel too slow or too fast on this device';

  @override
  String get deviceSlow => 'Slow';

  @override
  String get deviceNormal => 'Normal';

  @override
  String get deviceFast => 'Fast';

  @override
  String get deviceNaturalTip =>
      'Natural scrolling is on here, so this device scrolls the natural way too. For regular mouse scrolling, turn on Reverse Vertical Scrolling.';

  @override
  String get deviceKeys => 'Keys';

  @override
  String get deviceModifiers => 'Modifier Keys';

  @override
  String get deviceModifierPreset => 'Modifier key layout';

  @override
  String get deviceMacHabit => 'Mac Style';

  @override
  String get deviceXkbHabit => 'PC / XKB Style';

  @override
  String get deviceKeyboardUnknown =>
      'The keyboard types are not known yet, so modifier keys are sent as is. This adjusts automatically after the devices exchange system information.';

  @override
  String get deviceWindowsHabit => 'Windows Style';

  @override
  String get deviceAsIs => 'As Is';

  @override
  String get deviceCustom => 'Custom';

  @override
  String get deviceOtherKeys => 'Other Key Mappings';

  @override
  String get deviceOtherKeysHint => 'Turn one key into another, or send nothing';

  @override
  String get deviceTextMac => 'Mac-Style Text Editing Shortcuts';

  @override
  String get deviceTextXkb => 'PC / XKB Text Editing Shortcuts';

  @override
  String get deviceTextWindows => 'Windows-Style Text Editing Shortcuts';

  @override
  String get deviceTextMacHint => '⌘← ⌘→ go to line start and end, ⌥← ⌥→ move by word, ⌘⌫ deletes to line start';

  @override
  String get deviceTextXkbHint => 'Home and End go to line start and end, Ctrl+← Ctrl+→ move by word';

  @override
  String get deviceTextWindowsHint => 'Home and End go to line start and end, Ctrl+← Ctrl+→ move by word';

  @override
  String get deviceTextShortcuts => 'Text editing shortcuts';

  @override
  String get deviceSwitching => 'Switching & Sharing';

  @override
  String get deviceDwell => 'Prevent Accidental Switching';

  @override
  String get deviceDwellServerHint => 'The cursor pauses at the edge before switching to this device';

  @override
  String get deviceDwellClientHint => 'When this computer controls it, the cursor pauses at the edge before switching';

  @override
  String get deviceDwellLabel => 'How long to pause at the edge';

  @override
  String get deviceOff => 'Off';

  @override
  String deviceSeconds(String s) {
    return '$s s';
  }

  @override
  String get deviceClipboardHint => 'Sync copied content with this device';

  @override
  String get deviceClipboardOffHint => 'Clipboard sync is off in Settings';

  @override
  String get deviceClipboardLabel => 'Sync clipboard with this device';

  @override
  String get deviceDropFiles => 'Drag Files to It';

  @override
  String get deviceDropFilesHint => 'Drag files to the screen edge facing it to send them';

  @override
  String get deviceConnection => 'Connection';

  @override
  String get deviceThisDevice => 'This Device';

  @override
  String devicePresetMacDiffer(String name) {
    return 'On $name, ⌘C and ⌘V copy and paste';
  }

  @override
  String devicePresetWinDiffer(String name) {
    return 'On $name, Ctrl+C and Ctrl+V copy and paste';
  }

  @override
  String get devicePresetSame => 'Both use the same kind of keyboard, so keys are sent as is';

  @override
  String get devicePresetPlainMac => 'Sent as is: ⌘ becomes the Win key there';

  @override
  String get devicePresetPlainWin => 'Sent as is: the Win key becomes ⌘ there';

  @override
  String get devicePresetPlain => 'Sent as is';

  @override
  String get devicePresetCustom => 'Mapped one by one using the table below';

  @override
  String get deviceAlias => 'Name on This Computer';

  @override
  String get deviceAliasHint =>
      'Only changes the name shown here. The other computer keeps its own name. Leave it empty to use the original name.';

  @override
  String deviceOriginalName(String name) {
    return 'Original name: $name';
  }

  @override
  String deviceSystemFingerprint(String system) {
    return '$system · Fingerprint ';
  }

  @override
  String get deviceControlsThis => 'Controls this computer';

  @override
  String devicePlaceAdjust(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': 'Left of this computer',
      'right': 'Right of this computer',
      'top': 'Above this computer',
      'other': 'Below this computer',
    });
    return '$_temp0 · Adjust';
  }

  @override
  String get deviceControlled => 'This Computer Is Being Controlled';

  @override
  String deviceControlledHint(String name) {
    return 'Mouse, key and switching settings apply when this computer controls it. To change scrolling and keys while it controls this computer, use $name.';
  }

  @override
  String deviceOpenOn(String name) {
    return 'Open on $name';
  }

  @override
  String get devicePointerSpeed => 'Pointer Speed';

  @override
  String deviceBaseHere(int dpi) {
    return '$dpi · This computer';
  }

  @override
  String get deviceReset => 'Reset';

  @override
  String get deviceLocalPress => 'Pressed here';

  @override
  String devicePeerGets(String name) {
    return '$name receives';
  }

  @override
  String deviceModifierBecomes(String key, String name) {
    return '$key on $name becomes';
  }

  @override
  String get deviceDeleteMapping => 'Delete Mapping';

  @override
  String get deviceLastAddress => 'Last Address';

  @override
  String get deviceLastAddressServerHint => 'It last connected from this address';

  @override
  String get deviceLastAddressClientHint => 'Tried first when it can’t be found on the local network';

  @override
  String get deviceNotYet => 'None yet';

  @override
  String get deviceInvite => 'Invite It to Connect';

  @override
  String get deviceInviteOnlineHint => 'It’s already connected';

  @override
  String get deviceInviteHint =>
      'Use when it can’t find this computer on the network and has no keyboard and mouse nearby';

  @override
  String get deviceInviteButton => 'Invite';

  @override
  String get deviceConnectMode => 'How to Connect';

  @override
  String get deviceFixedHint => 'Only connect to the address below, without searching the network';

  @override
  String get deviceAutoHint => 'Search the network for the controlling computer, and fall back to the last address';

  @override
  String get deviceAuto => 'Automatic';

  @override
  String get deviceFixed => 'Fixed Address';

  @override
  String get deviceServerAddress => 'Controlling computer’s address';

  @override
  String get deviceAddressExample => 'e.g. 192.168.2.12 or 192.168.2.12:24810';

  @override
  String get deviceRestoreDefaults => 'Restore Defaults';

  @override
  String get deviceRestoreDefaultsHint => 'Mouse, keys, switching and sharing go back to defaults. The name is kept';

  @override
  String get deviceRestore => 'Restore';

  @override
  String get deviceRemove => 'Remove This Device';

  @override
  String get deviceRemoveConfirm => 'Are you sure? You’ll need to pair again, and its settings will be deleted';

  @override
  String get deviceRemoveHint => 'You’ll need to pair again after removing it';

  @override
  String get deviceRemoveButton => 'Remove';

  @override
  String get phStarted => 'ScreenFerry started';

  @override
  String phStartedServer(String name) {
    return '$name · Keyboard and mouse here';
  }

  @override
  String phStartedClient(String name) {
    return '$name · Controlled by another computer';
  }

  @override
  String get phCannotShare => 'Can’t start sharing';

  @override
  String get phListening => 'Sharing started';

  @override
  String phListeningAnnounced(String port) {
    return 'Waiting for another computer · Port $port';
  }

  @override
  String phListeningQuiet(String port) {
    return 'Port $port · Not announced on the local network';
  }

  @override
  String phConnected(String name) {
    return 'Connected to $name';
  }

  @override
  String phDisconnected(String name) {
    return 'Disconnected from $name';
  }

  @override
  String get phPeerClosed => 'The other computer disconnected';

  @override
  String phPeerOpenedSettings(String name) {
    return '$name asked to open settings here';
  }

  @override
  String get phShowingItsSettings => 'Showing its settings';

  @override
  String phCursorToPeer(String name) {
    return 'Cursor moved to $name';
  }

  @override
  String phAcrossEdge(String side) {
    String _temp0 = intl.Intl.selectLogic(side, {'left': 'left', 'right': 'right', 'top': 'top', 'other': 'bottom'});
    return 'Across the $_temp0 edge';
  }

  @override
  String get phCursorBack => 'Cursor back here';

  @override
  String phShortcut(String keys) {
    return 'Shortcut $keys';
  }

  @override
  String get phFromPeerEdge => 'Across the other screen’s edge';

  @override
  String get phCursorHere => 'Cursor arrived here';

  @override
  String phFrom(String name) {
    return 'From $name';
  }

  @override
  String phCursorReturned(String name) {
    return 'Cursor back on $name';
  }

  @override
  String phClipToPeer(String name) {
    return 'Clipboard synced to $name';
  }

  @override
  String get phClipHere => 'Clipboard synced here';

  @override
  String phClipTooBigToPeer(String format, String name) {
    String _temp0 = intl.Intl.selectLogic(format, {'image': 'image', 'text': 'text', 'other': 'formatted content'});
    return 'The $_temp0 on the clipboard is too large to sync to $name';
  }

  @override
  String phClipTooBigHere(String format, String name) {
    String _temp0 = intl.Intl.selectLogic(format, {'image': 'image', 'text': 'text', 'other': 'formatted content'});
    return 'The $_temp0 on $name’s clipboard is too large to sync here';
  }

  @override
  String phClipLimit(String size, String limit) {
    return '$size, over the $limit limit · You can raise it in Settings';
  }

  @override
  String phPaired(String name) {
    return 'Paired with $name';
  }

  @override
  String phRejectedByPeer(String name) {
    return '$name declined pairing';
  }

  @override
  String phRejected(String name) {
    return 'Declined the pairing request from $name';
  }

  @override
  String phPairingRequest(String name) {
    return '$name wants to pair';
  }

  @override
  String phPairingCode(String code) {
    return 'Pairing code $code';
  }

  @override
  String phItems(String file, int items, int more) {
    String _temp0 = intl.Intl.pluralLogic(more, locale: localeName, other: '$more more', one: '1 more');
    return '$file and $_temp0';
  }

  @override
  String phFiles(int files, String size) {
    String _temp0 = intl.Intl.pluralLogic(files, locale: localeName, other: '$files files', one: '1 file');
    return '$_temp0 · $size';
  }

  @override
  String phSending(String name, String what) {
    return 'Sending $what to $name';
  }

  @override
  String phReceiving(String name, String what) {
    return '$name is sending $what';
  }

  @override
  String phSent(String what) {
    return 'Sent $what';
  }

  @override
  String phSentTo(String name, String size) {
    return 'To $name · $size';
  }

  @override
  String phReceived(String what) {
    return 'Received $what';
  }

  @override
  String phSavedIn(String folder) {
    return 'Saved in $folder';
  }

  @override
  String phInterrupted(String what) {
    return 'Transfer interrupted: $what';
  }

  @override
  String get phResumesLater => 'Resumes after reconnecting';

  @override
  String phCancelled(String what) {
    return 'Cancelled: $what';
  }

  @override
  String get phCancelledByPeer => 'Cancelled by the other computer';

  @override
  String get phCancelledHere => 'Cancelled here';

  @override
  String phFailed(String what) {
    return 'Transfer failed: $what';
  }

  @override
  String get phSharingResumed => 'Sharing resumed';

  @override
  String get phSharingResumedHint => 'The keyboard and mouse can move to the other computer';

  @override
  String get phSharingPaused => 'Sharing paused';

  @override
  String get phSharingPausedHint => 'The keyboard and mouse stay here';

  @override
  String get phArranged => 'Rearranged devices';

  @override
  String get phLayoutSaved => 'Layout saved';

  @override
  String get phOtherDevices => 'Other devices';

  @override
  String phMoved(String name, String where) {
    return '$name moved $where';
  }

  @override
  String get phNowServer => 'Now controlling other computers';

  @override
  String get phNowServerHint => 'The keyboard and mouse are here. Waiting for another computer';

  @override
  String get phNowClient => 'Now controlled by another computer';

  @override
  String get phNowClientHint => 'Looking for the computer with the keyboard and mouse';

  @override
  String phRemoved(String name) {
    return 'Removed $name';
  }

  @override
  String get phPairAgain => 'Pair again to reconnect';

  @override
  String toastPeerSettingsOpened(String name) {
    return 'Settings opened on $name. Move the cursor over to make changes';
  }

  @override
  String toastPeerSettingsUnsupported(String name) {
    return 'ScreenFerry on $name is out of date. Open settings on that computer';
  }

  @override
  String get toastDiagnosticsMissing => 'The diagnostics file is gone. Export it again';

  @override
  String get toastNotConnectedPeer => 'Not connected, or the other computer can’t receive files';

  @override
  String toastFilesDisabled(String name) {
    return 'Dragging files to $name is turned off in settings';
  }

  @override
  String toastSending(String name) {
    return 'Sending to $name';
  }

  @override
  String get toastSharingResumed => 'Sharing resumed';

  @override
  String get toastSharingPaused => 'Sharing paused';

  @override
  String toastAutostartFailed(String message) {
    return 'Couldn’t change Start at Login: $message';
  }

  @override
  String toastSaveFailed(String message) {
    return 'Couldn’t save settings: $message';
  }

  @override
  String toastPeerOffline(String name) {
    return '$name isn’t connected right now';
  }

  @override
  String toastPeersReadFailed(String message) {
    return 'Couldn’t read paired devices: $message';
  }

  @override
  String toastRemoveFailed(String message) {
    return 'Couldn’t remove: $message';
  }

  @override
  String get toastServerGone => 'That computer is no longer on the local network';

  @override
  String get toastPausedConnectLater => 'Sharing is paused. Resume sharing, then connect';

  @override
  String get toastNotSharingInviteLater => 'Sharing hasn’t started yet. Invite once it has';

  @override
  String toastDiagnosticsFailed(String message) {
    return 'Couldn’t export diagnostics: $message';
  }

  @override
  String layoutDropAtNth(String side, int n) {
    String _temp0 = intl.Intl.selectLogic(side, {
      'left': 'left of this computer, number $n from the top',
      'right': 'right of this computer, number $n from the top',
      'top': 'above this computer, number $n from the left',
      'other': 'below this computer, number $n from the left',
    });
    return 'Release to place it $_temp0';
  }

  @override
  String aboutVersionLine(String version) {
    return 'Version $version · Share one keyboard and mouse across your computers';
  }

  @override
  String updateCurrent(String version) {
    return 'Current version $version';
  }

  @override
  String get updateUpToDate => 'You’re up to date';

  @override
  String updateCheckedAgo(String version, String ago) {
    return '$version · Checked $ago';
  }

  @override
  String get updateChecking => 'Checking for updates…';

  @override
  String get updateCheckingHint => 'Only fetches a small update file and sends nothing about your devices';

  @override
  String updateFound(String version) {
    return 'Version $version is available';
  }

  @override
  String updateReleased(String date) {
    return 'Released $date';
  }

  @override
  String get updateDownloadedHint => 'Downloaded in the background';

  @override
  String get updateDownload => 'Download';

  @override
  String updateDownloading(String version) {
    return 'Downloading $version';
  }

  @override
  String get updateRestartHint => 'The keyboard and mouse disconnect for about 5 seconds';

  @override
  String get updateLater => 'Later';

  @override
  String get updateRestart => 'Restart to Update';

  @override
  String get updateInstall => 'Install';

  @override
  String get updateLocalConfirm =>
      'Windows asks for administrator approval during installation, which can’t be clicked from another computer. Use this computer’s own keyboard and mouse to click Install';

  @override
  String updateInstalling(String version) {
    return 'Installing $version…';
  }

  @override
  String get updateInstallingHint => 'ScreenFerry will reopen by itself, and this window will reconnect';

  @override
  String updateManual(String version) {
    return 'Version $version needs to be installed manually';
  }

  @override
  String get updateManualHint =>
      'This copy can’t update itself. It may be a self-built development version, or in a folder you can’t write to';

  @override
  String get updateOpenDownloads => 'Open Download Page';

  @override
  String get updateUnreachable => 'Can’t reach the update server';

  @override
  String get updateUnreachableHint => 'System proxy settings were used. You can also download in a browser';

  @override
  String get updateBrowserDownload => 'Download in Browser';

  @override
  String get updateRetry => 'Retry';

  @override
  String get updateVerifyFailed => 'The download failed verification and was deleted';

  @override
  String get updateVerifyFailedHint =>
      'Nothing was installed. Something on the network may have altered the file. Try another network';

  @override
  String get updateStale => 'Can’t confirm you’re up to date';

  @override
  String get updateStaleHint => 'The update information is out of date, and something may be blocking new versions';

  @override
  String get updateNoKeys => 'This copy can’t check for updates';

  @override
  String get updateNoKeysHint =>
      'It has no built-in release key, which usually means it’s a self-built development version';

  @override
  String get updateInstallFailed => 'The update wasn’t installed';

  @override
  String get updateBreaksPeers => 'Update both computers';

  @override
  String updateBreaksPeersHint(String version) {
    return '$version changes how the two computers talk to each other. The other computer must also update to $version to reconnect';
  }

  @override
  String get updateAuto => 'Check for Updates Automatically';

  @override
  String get updateAutoHint =>
      'Checks once a day and downloads new versions in the background. You decide when to restart';

  @override
  String updateLastChecked(String ago) {
    return 'Last checked $ago';
  }

  @override
  String updateSidebar(String version) {
    return 'Update to $version';
  }

  @override
  String updateBannerTitle(String version) {
    return 'Version $version is ready';
  }

  @override
  String get updateBannerHint =>
      'Restart ScreenFerry to finish updating. The keyboard and mouse disconnect for about 5 seconds';

  @override
  String devicePeerCanUpdate(String version) {
    return 'Can update to $version';
  }

  @override
  String phUpdateInstalled(String to) {
    return 'Updated to $to';
  }

  @override
  String phFromVersion(String from) {
    return 'From $from';
  }

  @override
  String phUpdateNotInstalled(String version) {
    return '$version wasn’t installed';
  }

  @override
  String phStillOn(String current) {
    return 'Still on $current';
  }

  @override
  String get toastUpdateCancelled => 'Administrator approval wasn’t given, so nothing was installed';

  @override
  String toastUpdateFailed(String message) {
    return 'The update wasn’t installed: $message';
  }
}
