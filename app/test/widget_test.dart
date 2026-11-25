import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:screenferry_settings/app.dart';
import 'package:screenferry_settings/shell/bottom_nav.dart';
import 'package:screenferry_settings/shell/sidebar.dart';
import 'package:screenferry_settings/state/app_state.dart';
import 'package:screenferry_settings/theme/tokens.dart';
import 'package:screenferry_settings/widgets/address_dialog.dart';
import 'package:screenferry_settings/widgets/brand.dart';
import 'package:screenferry_settings/widgets/controls.dart';
import 'package:screenferry_settings/widgets/glyph.dart';
import 'package:screenferry_settings/widgets/group.dart';
import 'package:screenferry_settings/widgets/key_map_dialog.dart';
import 'package:screenferry_settings/widgets/page_scaffold.dart';
import 'package:screenferry_settings/widgets/surface.dart';

import 'support.dart';

/// 花朵一直在转，等不到“全部动画停下”，所以按固定时长推进。
Future<void> settle(WidgetTester tester, {int steps = 6}) async {
  for (var i = 0; i < steps; i++) {
    await tester.pump(const Duration(milliseconds: 150));
  }
}

/// 用演示后台启动界面。默认不让演示数据自己跳动，测试结果才稳定。
/// 测试里不真的打开浏览器：[opened] 记下打开过的链接。
/// [system]：系统语言，默认中文。
Future<AppState> launch(
  WidgetTester tester,
  Size size, {
  Backend? backend,
  List<String>? opened,
  Locale system = const Locale('zh'),
}) async {
  useLocale(tester, system);
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final state = AppState(
    backend: backend ?? DemoBackend(animate: false),
    openLink: (url) async {
      opened?.add(url);
      return opened != null;
    },
  );
  addTearDown(state.dispose);
  await tester.pumpWidget(ScreenFerryApp(state: state));
  state.start();
  await settle(tester);
  return state;
}

Future<void> visitAll(WidgetTester tester, AppState state) async {
  for (final page in [SfPage.devices, SfPage.layout, SfPage.log, SfPage.settings, SfPage.about, SfPage.overview]) {
    state.go(page);
    await settle(tester);
  }
  // 设备的设置页。
  state.openDevice(state.peers.first.fingerprint);
  await settle(tester);
  state.go(SfPage.overview);
  await settle(tester);
}

/// 把 [finder] 滚进视野，并让它离开顶部固定的标题栏（标题栏会挡住点击），再点它。
Future<void> tapInView(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await settle(tester, steps: 2);
  final top = tester.getRect(finder.first).top;
  if (top < 160) {
    await tester.drag(find.byType(Scrollable).first, Offset(0, 160 - top));
    await settle(tester, steps: 2);
  }
  await tester.tap(finder);
  await settle(tester);
}

/// 设置行 [title] 上的开关。
Finder switchOf(String title) =>
    find.descendant(of: find.widgetWithText(SfRow, title), matching: find.byType(SfSwitch));

Finder navItem(String label) => find.descendant(of: find.byType(Sidebar), matching: find.text(label));

/// 布局页里本机（`local`）或某台设备（证书指纹）的框。
Rect screenRect(WidgetTester tester, String id) => tester.getRect(find.byKey(ValueKey('screen-$id')));

String fingerprintOf(AppState state, String name) => state.peers.firstWhere((p) => p.name == name).fingerprint;

/// 打开布局页，返回演示数据里三台设备的指纹：办公室 PC、书房 PC（都在右侧）、家里的笔记本（左侧）。
Future<(String, String, String)> openLayout(WidgetTester tester, AppState state) async {
  state.go(SfPage.layout);
  await settle(tester);
  return (fingerprintOf(state, '办公室 PC'), fingerprintOf(state, '书房 PC'), fingerprintOf(state, '家里的笔记本'));
}

/// 页面大标题所在标题栏的底色。
Color headerColor(WidgetTester tester, String title) {
  final text = find.descendant(of: find.byType(CustomScrollView), matching: find.text(title)).first;
  return tester.widget<Container>(find.ancestor(of: text, matching: find.byType(Container)).first).color!;
}

void main() {
  for (final (name, size) in [
    ('默认窗口', const Size(1160, 780)),
    ('最小窗口', const Size(880, 600)),
    ('窄屏', const Size(600, 900)),
    ('手机大小', const Size(375, 667)),
  ]) {
    testWidgets('$name下所有页面都能正常显示', (tester) async {
      final state = await launch(tester, size);
      expect(find.text('仪表盘'), findsWidgets);
      await visitAll(tester, state);
      expect(find.text('往返延迟'), findsOneWidget);
    });

    testWidgets('$name下所有页面用英文也能正常显示', (tester) async {
      final state = await launch(tester, size, system: const Locale('en', 'US'));
      expect(find.text('Dashboard'), findsWidgets);
      await visitAll(tester, state);
      expect(find.text('Round-Trip Time'), findsOneWidget);
    });
  }

  testWidgets('点花朵中心暂停与继续共享', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    await tester.tap(find.byTooltip('暂停共享'));
    await settle(tester);
    expect(state.sharing, isFalse);
    expect(find.text('已暂停'), findsOneWidget);

    await tester.tap(find.byTooltip('继续共享'));
    await settle(tester);
    expect(state.sharing, isTrue);
  });

  testWidgets('客户端配对局域网里发现的服务端', (tester) async {
    final backend = DemoBackend(role: Role.client, animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    state.go(SfPage.devices);
    await settle(tester);

    await tester.ensureVisible(find.text('配对'));
    await settle(tester);
    await tester.tap(find.text('配对'));
    await settle(tester);
    expect(backend.sent.whereType<ConnectRequest>(), hasLength(1));
    expect(find.text('与“会议室 Mac”配对'), findsOneWidget);

    await tester.tap(find.text('数字一致，配对'));
    await settle(tester, steps: 12);
    expect(backend.sent.whereType<PairReplyRequest>().single.accept, isTrue);
    expect(state.peers.first.name, '会议室 Mac');
    expect(state.found, isEmpty);
    expect(find.text('与“会议室 Mac”配对'), findsNothing, reason: '配对结束后对话框自动关闭');
    expect(find.text('局域网里没有找到其他设备'), findsOneWidget);
  });

  testWidgets('服务端不列出局域网设备，只说明怎么配对', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.devices);
    await settle(tester);
    expect(find.text('等另一台电脑来找本机'), findsOneWidget);
    expect(find.text('配对'), findsNothing);
  });

  testWidgets('在设备的设置页移除设备，要先确认', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.devices);
    await settle(tester);
    await tester.tap(find.text('办公室 PC'));
    await settle(tester);
    expect(state.openedDevice, fingerprintOf(state, '办公室 PC'));

    await tapInView(tester, find.text('移除'));
    expect(find.textContaining('确定吗？'), findsOneWidget);
    expect(state.peers, hasLength(3), reason: '确认前不能移除');

    await tester.tap(find.text('移除'));
    await settle(tester);
    expect(state.peers.map((d) => d.name), isNot(contains('办公室 PC')));
    expect(state.openedDevice, isNull, reason: '移除后回到设备列表');
  });

  testWidgets('日志可以筛选和清空', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.log);
    await settle(tester);

    final total = state.log.length;
    expect(find.text('共 $total 条'), findsOneWidget);
    await tester.tap(find.text('剪贴板').first);
    await settle(tester);
    final clips = state.log.where((e) => e.kind == LogKind.clip).length;
    expect(find.text('共 $clips 条'), findsOneWidget);

    await tester.tap(find.text('清空'));
    await settle(tester);
    expect(find.text('暂无记录'), findsOneWidget);
  });

  testWidgets('切换深色主题和强调色', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.settings);
    await settle(tester);

    await tester.tap(find.text('深色'));
    await settle(tester);
    expect(state.appearance.mode, ThemeMode.dark);

    await tester.tap(find.byTooltip('紫色'));
    await settle(tester);
    expect(state.appearance.accent, 2);
  });

  testWidgets('点侧栏菜单切换页面', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    for (final (label, page) in [
      ('设备', SfPage.devices),
      ('布局', SfPage.layout),
      ('日志', SfPage.log),
      ('设置', SfPage.settings),
      ('仪表盘', SfPage.overview),
    ]) {
      await tester.tap(navItem(label));
      await settle(tester);
      expect(state.page, page);
    }
  });

  testWidgets('切换主题后标题栏跟着换色', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.settings);
    await settle(tester);
    expect(headerColor(tester, '设置'), SfColors.light.bg);

    await tester.tap(find.text('深色'));
    await settle(tester);
    expect(headerColor(tester, '设置'), SfColors.dark.bg);

    await tester.tap(find.text('浅色'));
    await settle(tester);
    expect(headerColor(tester, '设置'), SfColors.light.bg);
  });

  testWidgets('切换页面时新旧两页不会同时可见', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.devices);
    var sawOld = false;
    var sawNew = false;
    for (var ms = 0; ms < 420; ms += 20) {
      await tester.pump(const Duration(milliseconds: 20));
      final pages = find.byType(PageScaffold).evaluate().toList();
      final visible = [
        for (final page in pages)
          tester
              .widget<Opacity>(
                find.ancestor(of: find.byElementPredicate((e) => e == page), matching: find.byType(Opacity)).first,
              )
              .opacity,
      ].where((o) => o > 0.01).length;
      expect(visible, lessThanOrEqualTo(1), reason: '第 $ms 毫秒有 $visible 页同时可见');
      if (pages.length == 2) sawOld = true;
      if (pages.length == 1) sawNew = true;
    }
    expect(sawOld && sawNew, isTrue);
  });

  testWidgets('鼠标移开菜单项时底色不会先变暗', (tester) async {
    await launch(tester, const Size(1160, 780));
    // 悬停色本身比侧栏底色略深；渐变过程中不能比悬停色更暗。
    final side = SfColors.light.side;
    final floor = SfColors.light.navHover.computeLuminance() - 0.005;
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    addTearDown(mouse.removePointer);

    final item = navItem('布局');
    final box = find.ancestor(of: item, matching: find.byType(AnimatedContainer)).first;
    Color fill() {
      final deco = tester.widget<DecoratedBox>(find.descendant(of: box, matching: find.byType(DecoratedBox)).first);
      return Color.alphaBlend((deco.decoration as ShapeDecoration).color!, side);
    }

    var sawHover = false;
    for (final target in [tester.getCenter(item), const Offset(700, 400)]) {
      await mouse.moveTo(target);
      for (var i = 0; i < 12; i++) {
        await tester.pump(const Duration(milliseconds: 20));
        if (fill() != side) sawHover = true;
        expect(fill().computeLuminance(), greaterThan(floor));
      }
    }
    // 确认真的出现过悬停色，测试不是空跑。
    expect(sawHover, isTrue);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));

  for (final (name, size, wide) in [('宽窗口', const Size(1160, 780), true), ('窄屏', const Size(600, 900), false)]) {
    testWidgets('$name下横幅按钮的位置', (tester) async {
      final state = await launch(tester, size);
      for (final (page, label) in [(SfPage.devices, '复制指纹'), (SfPage.about, '检查更新')]) {
        state.go(page);
        await settle(tester);
        final banner = tester.getRect(find.byType(BannerRow));
        final button = tester.getRect(find.ancestor(of: find.text(label), matching: find.byType(PillButton)));
        if (wide) {
          // 按钮贴着横幅右边，和左边的图标一样留出卡片内边距。
          expect(button.right, moreOrLessEquals(banner.right), reason: label);
          expect(button.center.dy, moreOrLessEquals(banner.center.dy, epsilon: 1), reason: label);
        } else {
          expect(button.left, moreOrLessEquals(banner.left), reason: label);
          expect(button.bottom, moreOrLessEquals(banner.bottom), reason: label);
        }
      }
    });
  }

  for (final (name, size) in [('点按钮收起', const Size(1160, 780)), ('窗口太窄自动收起', const Size(600, 900))]) {
    testWidgets('侧栏$name后图标和状态圆点都居中', (tester) async {
      final state = await launch(tester, size);
      if (!state.sidebarCollapsed && size.width >= 900) {
        await tester.tap(find.byTooltip('收起侧栏'));
        await settle(tester);
      }
      final side = tester.getRect(find.byType(Sidebar));
      expect(side.width, collapsedWidth);
      // 内容区左右各留 14，右边还有 1 点分隔线。
      final center = side.left + 14 + (collapsedWidth - 29) / 2;
      final marks = [
        ...find.descendant(of: find.byType(Sidebar), matching: find.byType(GlyphIcon)).evaluate(),
        ...find.descendant(of: find.byType(Sidebar), matching: find.byType(StatusDot)).evaluate(),
      ];
      expect(marks.length, greaterThanOrEqualTo(6));
      for (final e in marks) {
        expect(tester.getCenter(find.byElementPredicate((x) => x == e)).dx, moreOrLessEquals(center), reason: '$e');
      }
    });
  }

  testWidgets('手机大小时用底部标签栏切换页面', (tester) async {
    final state = await launch(tester, const Size(375, 667));
    expect(find.byType(Sidebar), findsNothing);
    for (final (label, page) in [('设备', SfPage.devices), ('日志', SfPage.log), ('仪表盘', SfPage.overview)]) {
      await tester.tap(find.descendant(of: find.byType(BottomNav), matching: find.text(label)));
      await settle(tester);
      expect(state.page, page);
    }

    // 窗口拉宽后换回侧栏，当前页面不变。
    tester.view.physicalSize = const Size(1160, 780);
    await settle(tester);
    expect(find.byType(BottomNav), findsNothing);
    expect(find.byType(Sidebar), findsOneWidget);
    expect(state.page, SfPage.overview);
  });

  testWidgets('缺少权限时显示提示条，按钮请后台打开系统设置', (tester) async {
    final backend = DemoBackend(
      animate: false,
      problem: const Problem(kind: ProblemKind.permission, message: '需要在“隐私与安全性”中允许 ScreenFerry 控制电脑'),
    );
    await launch(tester, const Size(1160, 780), backend: backend);
    expect(find.text('需要键鼠控制权限'), findsOneWidget);
    expect(find.text('暂时无法共享'), findsOneWidget, reason: '主卡也说明了状态');

    await tester.tap(find.text('打开系统设置'));
    await settle(tester);
    expect(backend.sent.whereType<OpenPermissionSettingsRequest>(), hasLength(1));

    backend.problem = null;
    await settle(tester);
    expect(find.text('需要键鼠控制权限'), findsNothing);
  });

  testWidgets('对方发起配对时弹出配对码，后台说结束就自动关闭', (tester) async {
    final backend = DemoBackend(animate: false);
    await launch(tester, const Size(1160, 780), backend: backend);

    final id = backend.requestPairing('会议室 PC');
    await settle(tester);
    expect(find.text('与“会议室 PC”配对'), findsOneWidget);
    // 只在对话框里找：首页的统计数字随时间变化，也可能恰好是 4。
    final dialog = find.byWidgetPredicate((w) => w.runtimeType.toString() == '_PairingDialog');
    expect(
      find.descendant(of: dialog, matching: find.text('4')),
      findsOneWidget,
      reason: '配对码逐位显示',
    );

    backend.emit(PairingClosedNotice(id));
    await settle(tester);
    expect(find.text('与“会议室 PC”配对'), findsNothing);
  });

  testWidgets('拒绝配对会告诉后台并关闭对话框', (tester) async {
    final backend = DemoBackend(animate: false);
    await launch(tester, const Size(1160, 780), backend: backend);
    final id = backend.requestPairing('会议室 PC');
    await settle(tester);

    await tester.tap(find.text('拒绝'));
    await settle(tester);
    final reply = backend.sent.whereType<PairReplyRequest>().single;
    expect((reply.id, reply.accept), (id, false));
    expect(find.text('与“会议室 PC”配对'), findsNothing);
  });

  testWidgets('后台没有运行时显示占位页，侧栏仍在', (tester) async {
    final state = await launch(tester, const Size(1160, 780), backend: DemoBackend(animate: false, startOnline: false));
    expect(find.text('后台没有运行'), findsOneWidget);
    expect(find.text('请先启动 ScreenFerry，连上后会自动显示。'), findsOneWidget);
    expect(navItem('设备'), findsOneWidget);
    expect(find.text('往返延迟'), findsNothing);
    expect(state.online, isFalse);
  });

  testWidgets('与后台断开后保留数据、提示重连，并禁用会发命令的控件', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    expect(find.text('往返延迟'), findsOneWidget);

    backend.goOffline();
    await settle(tester);
    expect(find.text('与后台的连接已断开，正在重连…'), findsOneWidget);
    expect(find.text('往返延迟'), findsOneWidget, reason: '保留最后的数据');

    final sentBefore = backend.sent.length;
    await tester.tap(find.text('剪贴板同步'));
    await tester.tap(find.byTooltip('暂停共享'));
    await settle(tester);
    expect(backend.sent, hasLength(sentBefore), reason: '断开时不发命令');
    expect(state.settings.clipboard, isTrue);

    backend.goOnline();
    await settle(tester);
    expect(find.text('与后台的连接已断开，正在重连…'), findsNothing);
  });

  testWidgets('设置开关立即生效，并把改动发给后台', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    state.go(SfPage.settings);
    await settle(tester);

    // 同名的有标题文字和开关两处，开关在后。
    await tester.tap(find.bySemanticsLabel('同步剪贴板').last);
    await settle(tester);
    expect(state.settings.clipboard, isFalse);
    final patch = backend.sent.whereType<UpdateSettingsRequest>().last.settings;
    expect(patch.toJson(), {'clipboard': false});
  });
  testWidgets('拖动剪贴板上限滑块，松手才把新上限发给后台', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    state.go(SfPage.settings);
    await settle(tester);
    expect(find.text('32 MB'), findsOneWidget);

    final before = backend.sent.whereType<UpdateSettingsRequest>().length;
    final slider = find.byType(Slider);
    await tester.ensureVisible(slider);
    await settle(tester);
    // 拖到最左边：1 MB。
    await tester.drag(slider, const Offset(-400, 0));
    await settle(tester);
    expect(find.text('1 MB'), findsOneWidget);
    final sent = backend.sent.whereType<UpdateSettingsRequest>().toList();
    expect(sent.length, before + 1, reason: '整个拖动只发一次');
    expect(sent.last.settings.toJson(), {'clipboard_limit_mb': 1});
    expect(state.settings.clipboardLimitMb, 1);
  });

  for (final (name, path) in [('有预览文件', '/Users/me/blobs/ab/cdef'), ('没能存下预览', null)]) {
    testWidgets('同步过来的是图片时，总览显示图片的尺寸（$name）', (tester) async {
      final backend = DemoBackend(
        animate: false,
        clipboard: ClipboardSync(
          toPeer: false,
          peer: '办公室 PC',
          chars: null,
          bytes: 2400000,
          preview: '',
          image: ClipboardImage(width: 1920, height: 1080, path: path),
          at: DateTime.now(),
        ),
      );
      await launch(tester, const Size(1160, 780), backend: backend);
      await settle(tester);
      expect(find.text('一张图片'), findsOneWidget);
      expect(find.textContaining('图片 · 1920×1080'), findsOneWidget);
      if (path == null) {
        expect(find.byIcon(Icons.image_outlined), findsOneWidget, reason: '没有预览就画图片图标');
      } else {
        expect(find.bySemanticsLabel('剪贴板图片 1920×1080'), findsOneWidget);
      }
    });
  }

  testWidgets('分组卡片按圆角裁剪内容，行的悬停底色不会从圆角溢出', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    state.go(SfPage.settings);
    await settle(tester);
    final groups = find.byType(SfGroup);
    expect(groups, findsWidgets);
    for (final group in groups.evaluate()) {
      final clip = find.descendant(of: find.byWidget(group.widget), matching: find.byType(ClipPath)).first;
      expect(tester.widget<ClipPath>(clip).clipper, isA<ShapeBorderClipper>());
    }
  });

  testWidgets('在设置里切换角色，把新角色发给后台', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    state.go(SfPage.settings);
    await settle(tester);
    expect(find.text('本机鼠标的 DPI'), findsOneWidget, reason: '服务端可以填本机鼠标的 DPI');

    await tester.tap(find.text('被控制'));
    await settle(tester);
    final patch = backend.sent.whereType<UpdateSettingsRequest>().last.settings;
    expect(patch.toJson(), {'role': 'client'});
  });

  test('Linux、Windows、macOS 和未知系统分别显示自己的图标与名字', () {
    expect(platformLabelOf('linux'), 'Linux');
    expect(platformIconOf('linux'), Icons.computer_outlined);
    expect(platformLabelOf('windows'), 'Windows');
    expect(platformIconOf('windows'), Icons.desktop_windows_outlined);
    expect(platformLabelOf('macos'), 'macOS');
    expect(platformIconOf('macos'), Icons.desktop_mac_outlined);
    expect(platformLabelOf(''), 'Unknown');
    expect(platformIconOf(''), Icons.devices_other_outlined);
    expect(keyboardKindOf('linux'), KeyboardKind.xkb);
    expect(keyboardKindOf('windows'), KeyboardKind.xkb);
    expect(keyboardKindOf('macos'), KeyboardKind.mac);
    expect(keyboardKindOf(''), KeyboardKind.unknown);
  });

  testWidgets('Linux 当服务端时按 XKB 习惯显示，对端系统是 macOS', (tester) async {
    final backend = DemoBackend(animate: false, platform: 'linux');
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    expect(state.localIsMac, isFalse);
    expect(state.peerIsMac(state.peers.first), isTrue);
    expect(state.localIcon, Icons.computer_outlined);
    state.openDevice(fingerprintOf(state, '办公室 PC'));
    await settle(tester);
    expect(find.text('PC / XKB 习惯'), findsOneWidget);
    expect(find.text('文本编辑快捷键按 PC / XKB 习惯'), findsOneWidget);
    expect(find.text('Home End 到行首行尾，Ctrl+← Ctrl+→ 按词移动'), findsOneWidget);
    expect(find.textContaining('macOS · 指纹', findRichText: true), findsOneWidget);
    expect(state.localName, 'Arch 主机');
    expect(state.dataDir, '~/.local/share/screenferry');
  });

  testWidgets('未知系统不推断修饰键和文本快捷键的转换', (tester) async {
    final backend = DemoBackend(animate: false, platform: 'unknown');
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    final peer = state.peers.first;
    expect(state.localKeyboard, KeyboardKind.unknown);
    expect(state.peerKeyboard(peer), KeyboardKind.unknown);
    expect(state.keyboardsDiffer(peer), isNull);

    state.openDevice(peer.fingerprint);
    await settle(tester);
    expect(find.text('还不知道两台设备的键盘类型，修饰键先按原样发送。连上并交换系统信息后会自动调整。'), findsWidgets);
    expect(find.text('文本编辑快捷键按 Mac 习惯'), findsNothing);
    expect(find.text('文本编辑快捷键按 Windows 习惯'), findsNothing);
    expect(find.text('文本编辑快捷键按 PC / XKB 习惯'), findsNothing);
  });

  testWidgets('Windows 当服务端时按 Windows 的说法显示', (tester) async {
    final backend = DemoBackend(animate: false, platform: 'windows');
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    expect(state.localIsMac, isFalse);
    state.openDevice(fingerprintOf(state, '书房 PC'));
    await settle(tester);
    expect(find.text('Windows 习惯'), findsOneWidget);
    expect(find.text('在 书房 PC 上按 Ctrl+C、Ctrl+V 就是复制、粘贴'), findsOneWidget);
    expect(find.text('Win'), findsOneWidget, reason: '本机的键按 Windows 的叫法');
    expect(find.text('⌘ Command'), findsOneWidget, reason: '对方是 Mac：Ctrl 到了那边是 ⌘');
    expect(find.text('文本编辑快捷键按 Windows 习惯'), findsOneWidget);
  });

  for (final (name, size) in [
    ('默认窗口', const Size(1160, 780)),
    ('窄屏', const Size(600, 900)),
    ('手机大小', const Size(375, 667)),
  ]) {
    testWidgets('$name下布局页把两台摆在本机右侧、一台摆在左侧，都在画布里且互不重叠', (tester) async {
      final state = await launch(tester, size);
      final (office, study, notebook) = await openLayout(tester, state);
      for (final name in ['办公室 PC', '书房 PC', '家里的笔记本']) {
        expect(find.text(name), findsOneWidget);
      }
      final local = screenRect(tester, 'local');
      final (a, b, n) = (screenRect(tester, office), screenRect(tester, study), screenRect(tester, notebook));
      expect(a.left, greaterThan(local.right));
      expect(b.left, greaterThan(local.right));
      expect(a.bottom, lessThan(b.top), reason: '同一侧按先后从上往下排');
      expect(n.right, lessThan(local.left));

      final stage = tester.getRect(
        find.ancestor(of: find.byKey(const ValueKey('screen-local')), matching: find.byType(Stack)).first,
      );
      final rects = [local, a, b, n];
      for (var i = 0; i < rects.length; i++) {
        expect(stage.contains(rects[i].topLeft) && stage.contains(rects[i].bottomRight), isTrue, reason: '第 $i 个框');
        for (var j = i + 1; j < rects.length; j++) {
          expect(rects[i].overlaps(rects[j]), isFalse, reason: '第 $i 和第 $j 个框重叠');
        }
      }
      expect(find.text('本机左侧 1 台、右侧 2 台。同一侧已连接的几台按先后平分那条边，光标从哪一段穿过去就到哪台'), findsOneWidget);
    });
  }

  testWidgets('把右侧的一台拖到左侧最上面，吸附过去，发出的 layout 按新的先后列出全部设备', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    final (office, study, notebook) = await openLayout(tester, state);
    final local = screenRect(tester, 'local');
    final from = screenRect(tester, study).center;
    // 左侧放两台时，第一台正对本机左边的上半段。
    final to = Offset(screenRect(tester, notebook).center.dx, local.top + local.height / 4);

    final gesture = await tester.startGesture(from);
    await gesture.moveBy(const Offset(-30, 0));
    await tester.pump();
    await gesture.moveTo(to);
    await tester.pump();
    expect(find.text('松手后放到本机左侧，从上往下第 1 台'), findsOneWidget, reason: '拖动时说明会落到哪里');
    await gesture.up();
    await settle(tester);

    final layout = backend.sent.whereType<UpdateSettingsRequest>().last.settings.layout;
    expect(layout, [
      Placement(study, ScreenSide.left),
      Placement(notebook, ScreenSide.left),
      Placement(office, ScreenSide.right),
    ]);
    final moved = screenRect(tester, study);
    expect(moved.right, lessThan(screenRect(tester, 'local').left));
    expect(moved.bottom, lessThan(screenRect(tester, notebook).top));
    expect(state.placeOf(study), (side: ScreenSide.left, index: 0));
  });

  testWidgets('在同一侧拖动可以调整先后', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    final (office, study, notebook) = await openLayout(tester, state);
    final from = screenRect(tester, study).center;
    await tester.dragFrom(from, screenRect(tester, office).center - from);
    await settle(tester);

    expect(backend.sent.whereType<UpdateSettingsRequest>().last.settings.layout, [
      Placement(notebook, ScreenSide.left),
      Placement(study, ScreenSide.right),
      Placement(office, ScreenSide.right),
    ]);
    expect(screenRect(tester, study).bottom, lessThan(screenRect(tester, office).top));
  });

  testWidgets('拖回原处不发命令；与后台断开时拖不动', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    final (_, study, _) = await openLayout(tester, state);
    await tester.dragFrom(screenRect(tester, study).center, const Offset(4, 3));
    await settle(tester);
    expect(backend.sent.whereType<UpdateSettingsRequest>(), isEmpty);

    backend.goOffline();
    await settle(tester);
    final before = screenRect(tester, study);
    await tester.dragFrom(before.center, const Offset(-600, 0));
    await settle(tester);
    expect(backend.sent.whereType<UpdateSettingsRequest>(), isEmpty);
    expect(screenRect(tester, study), before);
  });

  testWidgets('光标所在的设备有标识，状态区分已连接和未连接', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    final (office, _, _) = await openLayout(tester, state);
    expect(find.text('光标在这里'), findsNothing);
    expect(find.text('已连接'), findsNWidgets(2));
    expect(find.text('未连接'), findsOneWidget);

    backend.cursor = office;
    await settle(tester);
    expect(find.text('光标在这里'), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp('^办公室 PC.*光标在这台上')), findsOneWidget);
    expect(state.cursorPeer?.name, '办公室 PC');
  });

  testWidgets('光标切到另一台电脑时，界面把光标设成隐藏；回来后恢复', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    final (office, _, _) = await openLayout(tester, state);
    Finder hiding() => find.byWidgetPredicate((w) => w is MouseRegion && w.cursor == SystemMouseCursors.none);
    expect(hiding(), findsNothing);

    backend.cursor = office;
    await settle(tester);
    expect(state.cursorOnPeer, isTrue);
    expect(hiding(), findsOneWidget);

    backend.cursor = null;
    await settle(tester);
    expect(hiding(), findsNothing);
  });

  testWidgets('被控制的一方不隐藏光标，本机的鼠标照常能点界面', (tester) async {
    final backend = DemoBackend(role: Role.client, animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    backend.cursor = state.peers.first.fingerprint;
    await settle(tester);
    expect(state.cursorOnPeer, isTrue, reason: '光标在服务端那边');
    expect(find.byWidgetPredicate((w) => w is MouseRegion && w.cursor == SystemMouseCursors.none), findsNothing);
    // 本机的鼠标照常能点侧栏切换页面。
    await tester.tap(find.text('设置').first);
    await settle(tester);
    expect(state.page, SfPage.settings);
  });

  testWidgets('控制端光标切走时，隐藏光标的那层不挡点击', (tester) async {
    final backend = DemoBackend(animate: false);
    final state = await launch(tester, const Size(1160, 780), backend: backend);
    backend.cursor = state.peers.first.fingerprint;
    await settle(tester);
    expect(state.cursorOnPeer, isTrue);
    await tester.tap(find.text('设置').first);
    await settle(tester);
    expect(state.page, SfPage.settings);
  });

  testWidgets('只有一台设备时说明它在哪一侧，没有设备时提示去配对', (tester) async {
    final state = await launch(tester, const Size(1160, 780));
    final (_, study, notebook) = await openLayout(tester, state);
    for (final fp in [study, notebook]) {
      state.forget(state.peers.firstWhere((p) => p.fingerprint == fp));
    }
    await settle(tester);
    expect(find.text('办公室 PC 在本机右侧，光标从相接的那条边穿过去'), findsOneWidget);
  });

  testWidgets('还没有配对的设备时布局页只显示本机', (tester) async {
    final state = await launch(tester, const Size(1160, 780), backend: DemoBackend(animate: false, paired: false));
    state.go(SfPage.layout);
    await settle(tester);
    expect(find.byKey(const ValueKey('screen-local')), findsOneWidget);
    expect(find.textContaining('还没有配对的设备'), findsOneWidget);
    bool isDevice(Widget w) => switch (w.key) {
      ValueKey<String>(:final value) => value.startsWith('screen-') && value != 'screen-local',
      _ => false,
    };
    expect(find.byWidgetPredicate(isDevice), findsNothing, reason: '没有设备框');

    await tester.tap(find.text('去配对'));
    await settle(tester);
    expect(state.page, SfPage.devices);
  });

  // ── 每台设备单独设置（ADR 0009） ────────────────

  group('设备设置', () {
    Future<(AppState, DemoBackend)> openOffice(WidgetTester tester, {Size size = const Size(1160, 780)}) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, size, backend: backend);
      state.openDevice(fingerprintOf(state, '办公室 PC'));
      await settle(tester);
      return (state, backend);
    }

    DeviceSettings lastSent(DemoBackend backend) => backend.sent.whereType<SetDeviceRequest>().last.device;

    testWidgets('点设备卡片打开这台的设置页，返回回到列表', (tester) async {
      final state = await launch(tester, const Size(1160, 780));
      state.go(SfPage.devices);
      await settle(tester);
      expect(find.text('垂直滚动反向'), findsNothing, reason: '卡片上不列设置');

      await tester.tap(find.text('办公室 PC'));
      await settle(tester);
      expect(find.text('光标速度'), findsOneWidget);
      expect(find.text('修饰键'), findsOneWidget);
      expect(find.text('F13'), findsOneWidget, reason: '已有的按键映射');

      await tester.tap(find.byTooltip('返回设备列表'));
      await settle(tester);
      expect(state.openedDevice, isNull);
      expect(find.text('光标速度'), findsNothing);
    });

    testWidgets('改一项发出这台设备的整份设置，其他项保持不变', (tester) async {
      final (state, backend) = await openOffice(tester);
      await tapInView(tester, switchOf('水平滚动反向'));
      final d = lastSent(backend);
      expect(d.fingerprint, fingerprintOf(state, '办公室 PC'));
      expect(d.reverseHorizontal, isTrue);
      expect(d.reverseVertical, isTrue, reason: '原来打开的不受影响');
      expect(d.keys, hasLength(1));
      expect(state.deviceSettings(d.fingerprint).reverseHorizontal, isTrue, reason: '先按新值显示');
    });

    testWidgets('改对照表里的一个修饰键，方案变成自定义', (tester) async {
      final (_, backend) = await openOffice(tester);
      // Mac 控制 Windows：⌘ 本来变成 Ctrl。
      await tapInView(tester, find.byTooltip('⌘ Command 在 办公室 PC 上变成'));
      await tester.tap(find.text('Alt').last);
      await settle(tester);
      final d = lastSent(backend);
      expect(d.modifiers, ModifierPreset.custom);
      expect(d.customModifiers.gui, ModifierTarget.alt);
      expect(d.customModifiers.ctrl, ModifierTarget.gui, reason: '其余的从原来的对应关系开始');
    });

    testWidgets('光标速度可以直接填 DPI，“还原”回到和本机一样', (tester) async {
      final (state, backend) = await openOffice(tester);
      await tester.enterText(find.bySemanticsLabel('光标速度').last, '2000');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await settle(tester);
      expect(lastSent(backend).pointerDpi, 2000);
      expect(find.text('1.25×'), findsOneWidget, reason: '本机 1600 DPI');

      await tapInView(tester, find.text('还原'));
      expect(lastSent(backend).pointerDpi, 0);
    });

    testWidgets('添加按键映射：录一个键，选对方收到的键', (tester) async {
      final (_, backend) = await openOffice(tester);
      await tapInView(tester, find.text('添加'));
      expect(find.text('请在键盘上按一个键…'), findsOneWidget);

      await tester.sendKeyDownEvent(LogicalKeyboardKey.f12, physicalKey: PhysicalKeyboardKey.f12);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.f12, physicalKey: PhysicalKeyboardKey.f12);
      await settle(tester);
      expect(find.text('已录下'), findsOneWidget);

      // 对话框比窗口高时可以滚动。
      await tester.ensureVisible(find.text('Insert'));
      await tester.tap(find.text('Insert'));
      await settle(tester);
      expect(
        find.descendant(of: find.byType(KeyMapDialog), matching: find.text('办公室 PC 收到')),
        findsOneWidget,
        reason: '预览这条映射',
      );
      await tester.ensureVisible(find.widgetWithText(PillButton, '添加'));
      await tester.tap(find.widgetWithText(PillButton, '添加'));
      await settle(tester);
      final keys = lastSent(backend).keys;
      expect(keys, contains(const KeyMapping(0x00070045, 0x00070049)), reason: 'F12 → Insert');
      expect(keys, hasLength(2), reason: '原来的 F13 → Print Screen 还在');
    });

    testWidgets('给设备改名只改本机上显示的名字', (tester) async {
      final (state, backend) = await openOffice(tester);
      await tester.tap(find.byTooltip('改名字'));
      await settle(tester);
      await tester.enterText(find.byType(TextField).first, '会议室的电脑');
      await tester.tap(find.text('保存'));
      await settle(tester);
      expect(lastSent(backend).alias, '会议室的电脑');
      expect(find.text('原名 办公室 PC'), findsOneWidget);
      expect(state.peers.first.displayName, '会议室的电脑');
    });

    testWidgets('给设备起了名字后，总览和布局页都显示起的名字', (tester) async {
      final state = await launch(tester, const Size(1160, 780));
      final linked = state.peer!;
      state.setDevice(state.deviceSettings(linked.fingerprint).copyWith(alias: 'Windows 台式机'));
      await settle(tester);
      expect(state.peerName, 'Windows 台式机');
      expect(find.text('Windows 台式机'), findsWidgets, reason: '总览的状态卡片');
      expect(find.text(linked.name), findsNothing, reason: '不再显示它自己报的名字');

      state.go(SfPage.layout);
      await settle(tester);
      expect(find.text('Windows 台式机'), findsOneWidget);
    });

    testWidgets('在设备页给本机改名', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.devices);
      await settle(tester);
      await tester.tap(find.byTooltip('改名字').first);
      await settle(tester);
      await tester.enterText(find.byType(TextField).first, '书房 Mac');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await settle(tester);
      expect(backend.sent.whereType<UpdateSettingsRequest>().last.settings.toJson(), {'name': '书房 Mac'});
    });

    testWidgets('本机被控制时说明设置在哪里生效，并可以请对方打开设置', (tester) async {
      final backend = DemoBackend(role: Role.client, animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.openDevice(state.peers.first.fingerprint);
      await settle(tester);
      expect(find.text('本机现在是被控制的一方'), findsOneWidget);
      expect(find.text('本机控制它时生效'), findsNWidgets(2), reason: '鼠标、按键两节');

      await tester.tap(find.text('在 Mac Studio 上打开'));
      await settle(tester);
      expect(backend.sent.whereType<OpenPeerSettingsRequest>().single.fingerprint, state.peers.first.fingerprint);
    });

    testWidgets('另一台设备请本机打开设置时翻到它的设置页', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      final study = fingerprintOf(state, '书房 PC');
      backend.emit(ShowDeviceNotice(study));
      await settle(tester);
      expect(state.page, SfPage.devices);
      expect(state.openedDevice, study);
      expect(find.text('书房 PC'), findsWidgets);
    });

    testWidgets('设置页填本机鼠标的 DPI', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.settings);
      await settle(tester);
      await tester.enterText(find.bySemanticsLabel('本机鼠标的 DPI').last, '800');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await settle(tester);
      expect(backend.sent.whereType<UpdateSettingsRequest>().last.settings.toJson(), {'mouse_dpi': 800});
    });

    testWidgets('手机大小下设备设置页能正常显示', (tester) async {
      await openOffice(tester, size: const Size(375, 667));
      expect(tester.takeException(), isNull);
      await tester.scrollUntilVisible(find.text('移除这台设备'), 200, scrollable: find.byType(Scrollable).first);
      await settle(tester);
      expect(tester.takeException(), isNull);
    });
  });

  group('按地址连接', () {
    Finder inDialog(Finder f) => find.descendant(of: find.byType(AddressDialog), matching: f);

    Future<void> dial(WidgetTester tester, String address, String button) async {
      await tester.enterText(inDialog(find.byType(TextField)), address);
      await settle(tester, steps: 2);
      await tester.tap(inDialog(find.text(button)));
      await settle(tester);
    }

    testWidgets('被控端在设备页按地址连接，连上后关掉对话框', (tester) async {
      final backend = DemoBackend(role: Role.client, animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.devices);
      await settle(tester);
      expect(find.text('192.168.2.18'), findsOneWidget, reason: '本机地址');

      await tester.tap(find.text('按地址连接'));
      await settle(tester);
      expect(find.byType(AddressDialog), findsOneWidget);
      expect(find.text('输入控制端（键盘鼠标所在那台）的地址'), findsOneWidget);

      await dial(tester, ' 192.168.2.12 ', '连接');
      expect(backend.sent.whereType<ConnectAddressRequest>().single.address, '192.168.2.12');
      expect(find.byType(AddressDialog), findsNothing);
      expect(find.text('已连上 192.168.2.12'), findsOneWidget);
      expect(state.dial, isNull, reason: '关掉对话框后不再记着这次连接');
    });

    testWidgets('连不上时说明原因，可以改了地址重试', (tester) async {
      final backend = DemoBackend(role: Role.client, animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.devices);
      await settle(tester);
      await tester.tap(find.text('按地址连接'));
      await settle(tester);

      await dial(tester, '9.9.9.9', '连接');
      expect(find.byType(AddressDialog), findsOneWidget);
      expect(find.text('连不上 9.9.9.9'), findsOneWidget);
      expect(inDialog(find.text('重试')), findsOneWidget);

      await dial(tester, '192.168.2.12', '重试');
      expect(backend.sent.whereType<ConnectAddressRequest>().map((r) => r.address), ['9.9.9.9', '192.168.2.12']);
      expect(find.byType(AddressDialog), findsNothing);
    });

    testWidgets('控制端邀请一台电脑；点本机地址复制', (tester) async {
      final copied = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
        if (call.method == 'Clipboard.setData') copied.add((call.arguments as Map)['text'] as String);
        return null;
      });
      addTearDown(() => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, null));
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.devices);
      await settle(tester);

      await tester.tap(find.text('192.168.2.12'));
      await settle(tester);
      expect(copied, ['192.168.2.12']);
      expect(find.text('地址已复制'), findsOneWidget);

      await tester.tap(find.text('邀请一台电脑'));
      await settle(tester);
      expect(find.text('输入被控制那台的地址，它会自动连过来'), findsOneWidget);
      await dial(tester, '192.168.2.40', '邀请');
      expect(backend.sent.whereType<InviteRequest>().single.address, '192.168.2.40');
      expect(find.text('已邀请 192.168.2.40，它正在连过来'), findsOneWidget);
    });

    testWidgets('控制端在设备设置页邀请离线的设备，地址预先填好', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.openDevice(fingerprintOf(state, '办公室 PC'));
      await settle(tester);
      await tester.scrollUntilVisible(find.text('邀请它连过来'), 200, scrollable: find.byType(Scrollable).first);
      await settle(tester);
      expect(find.text('它已经连着本机'), findsOneWidget);
      final invite = find.descendant(of: find.widgetWithText(SfRow, '邀请它连过来'), matching: find.byType(SmallButton));
      expect(tester.widget<SmallButton>(invite).onPressed, isNull, reason: '在线的不用邀请');

      state.openDevice(fingerprintOf(state, '家里的笔记本'));
      await settle(tester);
      await tester.scrollUntilVisible(find.text('邀请它连过来'), 200, scrollable: find.byType(Scrollable).first);
      await settle(tester);
      expect(find.text('192.168.2.20'), findsOneWidget, reason: '上次连上的地址');
      await tapInView(tester, invite);
      expect(tester.widget<TextField>(inDialog(find.byType(TextField))).controller!.text, '192.168.2.20');
      await tester.tap(inDialog(find.text('邀请')));
      await settle(tester);
      expect(backend.sent.whereType<InviteRequest>().single.address, '192.168.2.20');
    });

    testWidgets('被控端在设备设置页选连接方式：固定地址或自动寻找', (tester) async {
      final backend = DemoBackend(role: Role.client, animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.openDevice(state.peers.single.fingerprint);
      await settle(tester);
      await tester.scrollUntilVisible(find.text('连接方式'), 200, scrollable: find.byType(Scrollable).first);
      await settle(tester);
      expect(find.text('192.168.2.12'), findsOneWidget, reason: '上次连上的地址');
      List<Object?> servers() => [
        for (final r in backend.sent.whereType<UpdateSettingsRequest>())
          if (r.settings.server != null) r.settings.server,
      ];

      await tapInView(tester, find.text('固定地址'));
      expect(servers(), ['192.168.2.12'], reason: '用上次连上的地址');
      expect(state.settings.server, '192.168.2.12');

      await tester.enterText(find.bySemanticsLabel('控制端的地址').last, '192.168.2.12:24900');
      await settle(tester, steps: 2);
      await tapInView(tester, find.text('保存'));
      expect(servers().last, '192.168.2.12:24900');

      await tapInView(tester, find.text('自动寻找'));
      expect(servers().last, '');
      expect(find.text('保存'), findsNothing);
    });

    testWidgets('手机大小下设备页和对话框能正常显示', (tester) async {
      final state = await launch(tester, const Size(375, 667), backend: DemoBackend(role: Role.client, animate: false));
      state.go(SfPage.devices);
      await settle(tester);
      await tapInView(tester, find.text('按地址连接'));
      await dial(tester, '9.9.9.9', '连接');
      expect(find.text('连不上 9.9.9.9'), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });

  testWidgets('后台要退出时设置界面跟着关闭', (tester) async {
    tester.view.physicalSize = const Size(1160, 780);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final backend = DemoBackend(animate: false);
    var quits = 0;
    useLocale(tester);
    final state = AppState(backend: backend, onQuit: () => quits++);
    addTearDown(state.dispose);
    await tester.pumpWidget(ScreenFerryApp(state: state));
    state.start();
    await settle(tester);
    expect(quits, 0);

    backend.emit(const QuitNotice());
    await settle(tester);
    expect(quits, 1);
  });

  group('关于', () {
    testWidgets('页头显示彩色的应用图标', (tester) async {
      final state = await launch(tester, const Size(1160, 780));
      state.go(SfPage.about);
      await settle(tester);
      expect(find.byType(AppIconMark), findsOneWidget);
      expect(find.bySemanticsLabel('ScreenFerry'), findsWidgets);
    });

    testWidgets('点作者主页用浏览器打开，不再列出加密和协议版本', (tester) async {
      final opened = <String>[];
      final state = await launch(tester, const Size(1160, 780), opened: opened);
      state.go(SfPage.about);
      await settle(tester);
      expect(find.text('加密与配对'), findsNothing);
      expect(find.text('协议版本'), findsNothing);

      await tester.tap(find.text('作者主页'));
      await settle(tester);
      expect(opened, ['https://github.com/icloudza']);
      expect(find.text('打不开浏览器，链接已复制'), findsNothing);
    });

    testWidgets('打不开浏览器时复制链接并提示', (tester) async {
      final copied = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
        if (call.method == 'Clipboard.setData') copied.add((call.arguments as Map)['text'] as String);
        return null;
      });
      addTearDown(() => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, null));
      final state = await launch(tester, const Size(1160, 780));
      state.go(SfPage.about);
      await settle(tester);

      await tester.tap(find.text('作者主页'));
      await settle(tester);
      expect(copied, ['https://github.com/icloudza']);
      expect(find.text('打不开浏览器，链接已复制'), findsOneWidget);
    });
  });

  group('运行日志', () {
    Future<(AppState, DemoBackend)> openRunLog(
      WidgetTester tester, {
      Size size = const Size(1160, 780),
      List<String>? opened,
    }) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, size, backend: backend, opened: opened);
      state.go(SfPage.log);
      await settle(tester);
      await tapInView(tester, find.text('运行日志'));
      return (state, backend);
    }

    /// 切回别的分段，停止定时读运行日志。
    Future<void> leave(WidgetTester tester) async {
      await tapInView(tester, find.text('全部').first);
    }

    testWidgets('打开时读最近的一段，之后只读新写的，切走就不再读', (tester) async {
      final (state, backend) = await openRunLog(tester);
      final reads = backend.sent.whereType<ReadRunLogRequest>().toList();
      expect(reads.single.since, isNull);
      expect(find.text('心跳超时：对方长时间没有响应'), findsOneWidget);
      expect(find.text('2.0 MB · 18,241 行'), findsOneWidget);
      expect(find.text('清空'), findsNothing, reason: '运行日志不能在这里清空');

      await tester.pump(const Duration(seconds: 2));
      expect(backend.sent.whereType<ReadRunLogRequest>().last.since, 2097152, reason: '接着上次的位置读');

      await leave(tester);
      final count = backend.sent.whereType<ReadRunLogRequest>().length;
      await tester.pump(const Duration(seconds: 5));
      expect(backend.sent.whereType<ReadRunLogRequest>().length, count);
      expect(state.runLog, isNotEmpty);
    });

    testWidgets('可以只看警告和错误', (tester) async {
      await openRunLog(tester);
      expect(find.text('等待客户端连接 addr=0.0.0.0:24810'), findsOneWidget);
      await tapInView(tester, find.text('警告和错误'));
      expect(find.text('等待客户端连接 addr=0.0.0.0:24810'), findsNothing);
      expect(find.text('连接中断：网络错误 · Connection reset by peer'), findsOneWidget);
      expect(find.text('心跳超时：对方长时间没有响应'), findsOneWidget);
      await leave(tester);
    });

    testWidgets('导出诊断文件后显示位置，可以在访达中显示；复制摘要到剪贴板', (tester) async {
      final copied = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
        if (call.method == 'Clipboard.setData') copied.add((call.arguments as Map)['text'] as String);
        return null;
      });
      addTearDown(() => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, null));
      final (_, backend) = await openRunLog(tester);

      await tapInView(tester, find.text('导出诊断文件'));
      expect(backend.sent.whereType<ExportDiagnosticsRequest>(), hasLength(1));
      expect(find.textContaining('ScreenFerry-诊断-20261102-0624.txt', findRichText: true), findsOneWidget);
      await tapInView(tester, find.text('在访达中显示'));
      expect(backend.sent.whereType<RevealDiagnosticsRequest>(), hasLength(1));

      await tapInView(tester, find.text('复制摘要'));
      expect(copied.single, startsWith('ScreenFerry 诊断摘要'));
      expect(find.text('已复制摘要：版本、系统和最近 200 行，可以直接贴进问题描述'), findsOneWidget);

      await tapInView(tester, find.text('打开日志文件夹'));
      expect(backend.sent.whereType<OpenLogsFolderRequest>(), hasLength(1));
      await leave(tester);
    });

    testWidgets('打开详细记录，提问题时带上版本和系统', (tester) async {
      final opened = <String>[];
      final (_, backend) = await openRunLog(tester, opened: opened);
      expect(find.textContaining('重启 ScreenFerry 后自动关闭'), findsOneWidget);
      await tapInView(tester, switchOf('详细记录'));
      expect(backend.sent.whereType<SetVerboseRequest>().single.on, isTrue);
      expect(find.textContaining('刚刚打开'), findsOneWidget);

      await tapInView(tester, find.text('去 GitHub 提问题'));
      final url = Uri.parse(opened.single);
      expect(url.path, '/icloudza/screen-ferry/issues/new');
      expect(url.queryParameters['template'], 'bug_report.yml');
      expect(url.queryParameters['version'], '0.1.0');
      expect(url.queryParameters['system'], 'macOS 15.6 · arm64');
      await leave(tester);
    });

    testWidgets('手机大小下运行日志能正常显示', (tester) async {
      await openRunLog(tester, size: const Size(375, 667));
      expect(tester.takeException(), isNull);
      await tester.scrollUntilVisible(find.text('最近的运行日志'), 200, scrollable: find.byType(Scrollable).first);
      await settle(tester);
      expect(tester.takeException(), isNull);
      await leave(tester);
    });
  });

  group('多语言', () {
    testWidgets('系统是英文时界面用英文，活动记录也按英文说', (tester) async {
      final state = await launch(tester, const Size(1160, 780), system: const Locale('en', 'US'));
      expect(navItem('Devices'), findsOneWidget);
      expect(find.text('Status'), findsOneWidget);

      state.go(SfPage.log);
      await settle(tester);
      expect(find.text('Across the right edge'), findsOneWidget);
      expect(find.textContaining('Shortcut Ctrl + '), findsOneWidget);
      expect(find.text('Clipboard synced here'), findsOneWidget);
      expect(find.textContaining('光标'), findsNothing);
    });

    testWidgets('在设置里选语言，界面马上换过去，并把选择发给后台', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend, system: const Locale('en'));
      state.go(SfPage.settings);
      await settle(tester);
      expect(find.text('Language'), findsOneWidget);

      await tester.tap(find.text('简体中文'));
      await settle(tester);
      expect(backend.sent.whereType<UpdateSettingsRequest>().last.settings.toJson(), {'language': 'zh'});
      expect(state.locale, const Locale('zh'));
      expect(find.text('语言'), findsOneWidget);

      await tester.tap(find.text('跟随系统').last);
      await settle(tester);
      expect(state.settings.language, UiLanguage.system);
      expect(find.text('Language'), findsOneWidget);
    });

    testWidgets('后台的提示按当前语言显示，认不出的照原话显示', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend, system: const Locale('en'));
      backend.emit(const ToastNotice('已暂停共享', phrase: Phrase('sharing_paused')));
      await tester.pump();
      expect(state.toast.value?.text, 'Sharing paused');
      backend.emit(const ToastNotice('后台新加的提示', phrase: Phrase('from_the_future')));
      await tester.pump();
      expect(state.toast.value?.text, '后台新加的提示');
      await settle(tester);
    });
  });

  group('版本更新', () {
    const release = UpdateRelease(
      version: '0.2.0',
      size: 21000000,
      notesZh: '界面有了英文\n运行日志与诊断导出\nWindows 安装包\n按地址连接\n设置界面只用一个窗口\n关于页更简洁',
      notesEn:
          'English UI\nDiagnostics log\nWindows installer\nConnect by address\nOne settings window\nSimpler About page',
    );

    Future<(AppState, DemoBackend)> about(WidgetTester tester, {Size size = const Size(1160, 780)}) async {
      final backend = DemoBackend(animate: false, offer: release);
      final state = await launch(tester, size, backend: backend);
      state.updateSettings(const SettingsPatch(autoUpdate: false));
      state.go(SfPage.about);
      await settle(tester);
      return (state, backend);
    }

    testWidgets('检查到新版本后手动下载，再重启更新', (tester) async {
      final (state, backend) = await about(tester);
      expect(find.text('当前版本 0.1.0'), findsOneWidget);

      await tester.tap(find.text('检查更新'));
      await settle(tester);
      expect(backend.sent.whereType<CheckUpdateRequest>(), hasLength(1));
      expect(find.text('发现新版本 0.2.0'), findsOneWidget);
      expect(find.text('界面有了英文'), findsOneWidget);

      await tester.tap(find.text('下载'));
      await settle(tester);
      expect(backend.sent.whereType<DownloadUpdateRequest>(), hasLength(1));
      expect(find.textContaining('已在后台下载好'), findsOneWidget);
      expect(find.text('重启时键鼠会断开约 5 秒'), findsOneWidget);

      await tapInView(tester, find.text('重启并更新'));
      expect(backend.sent.whereType<InstallUpdateRequest>(), hasLength(1));
      expect(state.updateInfo.status, UpdateStatus.installing);
      expect(find.text('正在安装 0.2.0…'), findsOneWidget);
    });

    testWidgets('下载好后侧栏和仪表盘都有提醒，点稍后收起横幅', (tester) async {
      final (state, backend) = await about(tester);
      await tester.tap(find.text('检查更新'));
      await settle(tester);
      await tester.tap(find.text('下载'));
      await settle(tester);

      final pill = find.descendant(of: find.byType(Sidebar), matching: find.text('更新到 0.2.0'));
      expect(pill, findsOneWidget);
      state.go(SfPage.overview);
      await settle(tester);
      expect(find.text('新版本 0.2.0 已准备好'), findsOneWidget);

      await tester.tap(find.descendant(of: find.byKey(const ValueKey('banner-update')), matching: find.text('稍后')));
      await settle(tester);
      expect(backend.sent.whereType<DismissUpdateRequest>().single.version, '0.2.0');
      expect(find.text('新版本 0.2.0 已准备好'), findsNothing);
      expect(pill, findsOneWidget, reason: '侧栏的提醒还在');

      await tester.tap(pill);
      await settle(tester);
      expect(state.page, SfPage.about);
    });

    testWidgets('关掉自动检查更新，把设置发给后台', (tester) async {
      final backend = DemoBackend(animate: false);
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.about);
      await settle(tester);
      expect(state.settings.autoUpdate, isTrue);
      await tester.tap(find.bySemanticsLabel('自动检查更新').last);
      await settle(tester);
      expect(state.settings.autoUpdate, isFalse);
      expect(backend.sent.whereType<UpdateSettingsRequest>().last.settings.toJson(), {'auto_update': false});
    });

    testWidgets('不能自己更新时请用户去下载页面', (tester) async {
      final opened = <String>[];
      final backend = DemoBackend(
        animate: false,
        update: const UpdateInfo(status: UpdateStatus.available, latest: release),
      );
      final state = await launch(tester, const Size(1160, 780), backend: backend, opened: opened);
      state.go(SfPage.about);
      await settle(tester);
      expect(find.text('发现新版本 0.2.0，要手动安装'), findsOneWidget);
      await tester.tap(find.text('打开下载页面'));
      await settle(tester);
      expect(opened, ['https://github.com/icloudza/screen-ferry/releases/latest']);
    });

    testWidgets('出错时说明原因', (tester) async {
      for (final (error, title) in [
        (UpdateError.unreachable, '连不上更新服务器'),
        (UpdateError.verifyFailed, '安装包没有通过校验，已删除'),
        (UpdateError.stale, '无法确认是不是最新版'),
        (UpdateError.noKeys, '这份程序不能检查更新'),
      ]) {
        final backend = DemoBackend(
          animate: false,
          update: UpdateInfo(status: UpdateStatus.failed, error: error),
        );
        final state = await launch(tester, const Size(1160, 780), backend: backend);
        state.go(SfPage.about);
        await settle(tester);
        expect(find.text(title), findsOneWidget, reason: '$error');
      }
    });

    testWidgets('被另一台电脑控制的 Windows 提醒在本机确认管理员权限', (tester) async {
      final backend = DemoBackend(
        role: Role.client,
        animate: false,
        update: const UpdateInfo(
          status: UpdateStatus.ready,
          latest: release,
          canInstall: true,
          needsLocalConfirm: true,
        ),
      );
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.about);
      await settle(tester);
      expect(find.textContaining('从另一台电脑操作时点不到它'), findsOneWidget);
      expect(find.text('安装'), findsOneWidget);
    });

    testWidgets('更新说明按新增、改进、修复分类显示', (tester) async {
      const categorized = UpdateRelease(
        version: '0.2.0',
        notesZh: '### 新增\n- Mac 安装盘\n### 修复\n- 修好偶尔卡键',
        notesEn: '### New\n- Mac installer disk\n### Fixed\n- Fixed stuck keys',
      );
      final backend = DemoBackend(
        animate: false,
        update: const UpdateInfo(status: UpdateStatus.ready, latest: categorized, canInstall: true),
      );
      final state = await launch(tester, const Size(1160, 780), backend: backend);
      state.go(SfPage.about);
      await settle(tester);
      for (final text in ['新增', 'Mac 安装盘', '修复', '修好偶尔卡键']) {
        expect(find.text(text), findsOneWidget, reason: text);
      }
      expect(find.textContaining('###'), findsNothing);
    });

    testWidgets('英文和窄屏下更新卡片能正常显示', (tester) async {
      final backend = DemoBackend(
        animate: false,
        update: const UpdateInfo(status: UpdateStatus.ready, latest: release, canInstall: true),
      );
      final state = await launch(tester, const Size(375, 667), backend: backend, system: const Locale('en'));
      state.go(SfPage.about);
      await settle(tester);
      expect(find.text('Version 0.2.0 is available'), findsOneWidget);
      expect(find.text('Restart to Update'), findsOneWidget);
    });
  });
}
