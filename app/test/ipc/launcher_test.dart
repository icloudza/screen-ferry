import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:screenferry_settings/ipc/launcher.dart';

void main() {
  late Directory root;

  setUp(() => root = Directory.systemTemp.createTempSync('sf-launcher-'));
  tearDown(() => root.deleteSync(recursive: true));

  String settingsExe(String app) => '$app/Contents/Helpers/ScreenFerry.app/Contents/MacOS/ScreenFerry';

  test('装在 ScreenFerry.app 里时找到外层应用', () {
    final app = '${root.path}/ScreenFerry.app';
    File('$app/Contents/MacOS/screenferry').createSync(recursive: true);
    expect(daemonLocation(settingsExe(app)), app);
  });

  test('外层应用里没有后台程序时找不到', () {
    final app = '${root.path}/ScreenFerry.app';
    Directory('$app/Contents/MacOS').createSync(recursive: true);
    expect(daemonLocation(settingsExe(app)), isNull);
  });

  test('Linux 安装目录里后台与设置界面相邻', () {
    final dir = '${root.path}/ScreenFerry';
    File('$dir/screenferry').createSync(recursive: true);
    expect(daemonLocation('$dir/screenferry-settings'), '$dir/screenferry');
  }, skip: !Platform.isLinux);

  test('开发时直接运行的界面不在安装包里', () {
    expect(
      daemonLocation('${root.path}/build/macos/Build/Products/Release/ScreenFerry.app/Contents/MacOS/ScreenFerry'),
      isNull,
    );
  });
}
