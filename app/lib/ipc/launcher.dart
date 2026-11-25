// 找到并启动后台。设置界面装在 ScreenFerry.app（Windows 上是安装目录）里时，后台就在旁边。

import 'dart:io';

/// 后台的位置：macOS 上是外层 ScreenFerry.app，Linux 和 Windows 上在同目录。
/// 不在安装包里（开发时直接运行）时为 null。
String? daemonLocation([String? executable]) {
  final exe = executable ?? Platform.resolvedExecutable;
  if (exe.contains('.app/')) {
    // …/ScreenFerry.app/Contents/Helpers/ScreenFerry.app/Contents/MacOS/ScreenFerry
    final parts = exe.split('/');
    final helpers = parts.lastIndexOf('Helpers');
    if (helpers < 2 || parts[helpers - 1] != 'Contents') return null;
    final app = parts.sublist(0, helpers - 1).join('/');
    if (!app.endsWith('.app')) return null;
    return File('$app/Contents/MacOS/screenferry').existsSync() ? app : null;
  }
  if (Platform.isLinux) {
    final daemon = File('${File(exe).parent.path}/screenferry');
    return daemon.existsSync() ? daemon.path : null;
  }
  if (Platform.isWindows) {
    // …\ScreenFerry\settings\screenferry-settings.exe，后台是 …\ScreenFerry\screenferry.exe
    final daemon = File('${File(exe).parent.parent.path}\\screenferry.exe');
    return daemon.existsSync() ? daemon.path : null;
  }
  return null;
}

/// 启动后台，成功发出启动命令时返回 true。之后的连接由客户端自动重试完成。
Future<bool> startDaemon(String location) async {
  try {
    if (Platform.isMacOS) {
      final result = await Process.run('open', [location]);
      return result.exitCode == 0;
    }
    await Process.start(location, const [], mode: ProcessStartMode.detached);
    return true;
  } on ProcessException {
    return false;
  }
}
