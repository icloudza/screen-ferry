// 一台设备的设置页（ADR 0009）：名字，鼠标，按键，切换与共享，连接（ADR 0010），恢复默认与移除。
// 鼠标、按键、防误切在本机控制它时生效（本机是服务端）。本机被控制时页面上方说明这一点，
// 并可以请对方打开它的设置界面、翻到本机的设置页。

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../l10n/l10n.dart';
import '../state/app_scope.dart';
import '../state/app_state.dart';
import '../state/key_names.dart';
import '../theme/tokens.dart';
import '../widgets/address_dialog.dart';
import '../widgets/controls.dart';
import '../widgets/dpi_field.dart';
import '../widgets/group.dart';
import '../widgets/key_map_dialog.dart';
import '../widgets/name_editor.dart';
import '../widgets/page_scaffold.dart';
import '../widgets/surface.dart';

class DevicePage extends StatelessWidget {
  const DevicePage({super.key, required this.peer});

  final PeerInfo peer;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final d = state.deviceSettings(peer.fingerprint);
    final server = state.isServer;
    final localKeyboard = state.localKeyboard;
    final peerKeyboard = state.peerKeyboard(peer);
    final localMac = localKeyboard == KeyboardKind.mac;
    final peerMac = peerKeyboard == KeyboardKind.mac;
    final keyboardDiffer = state.keyboardsDiffer(peer);
    final differ = keyboardDiffer ?? false;
    final keyboardUnknown = keyboardDiffer == null;
    final name = peer.displayName;
    // 没连上后台时控件不可用（返回 null）。
    ValueChanged<T>? change<T>(DeviceSettings Function(T) next) =>
        state.online ? (v) => state.setDevice(next(v)) : null;
    // 本机被控制时，键鼠方面的设置现在不起作用。
    final l = context.l10n;
    final later = server ? null : l.deviceLater;
    return PageScaffold(
      title: l.navDevices,
      leading: SfIconButton(icon: Icons.arrow_back_rounded, tooltip: l.deviceBack, onPressed: state.closeDevice),
      children: [
        _Header(peer: peer, settings: d),
        if (!server) _ControlledBanner(peer: peer),
        SectionTitle(l.deviceMouse, note: later),
        SfGroup(
          children: [
            _PointerSpeed(settings: d),
            SfRow(
              icon: Icons.swap_vert_rounded,
              title: l.deviceInvertVertical,
              subtitle: l.deviceInvertVerticalHint,
              trailing: SfSwitch(
                value: d.reverseVertical,
                label: l.deviceInvertVertical,
                onChanged: change((bool v) => d.copyWith(reverseVertical: v)),
              ),
            ),
            SfRow(
              icon: Icons.swap_horiz_rounded,
              title: l.deviceInvertHorizontal,
              subtitle: l.deviceInvertHorizontalHint,
              trailing: SfSwitch(
                value: d.reverseHorizontal,
                label: l.deviceInvertHorizontal,
                onChanged: change((bool v) => d.copyWith(reverseHorizontal: v)),
              ),
            ),
            Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SfRow(
                  icon: Icons.speed_rounded,
                  title: l.deviceScrollSpeed,
                  subtitle: l.deviceScrollSpeedHint,
                  trailing: SfSegmented<ScrollSpeed>(
                    label: l.deviceScrollSpeed,
                    segments: [
                      (ScrollSpeed.slow, l.deviceSlow),
                      (ScrollSpeed.normal, l.deviceNormal),
                      (ScrollSpeed.fast, l.deviceFast),
                    ],
                    value: d.scrollSpeed,
                    onChanged: change((ScrollSpeed v) => d.copyWith(scrollSpeed: v)),
                  ),
                ),
                if (server && state.naturalScrolling && peerKeyboard == KeyboardKind.xkb) _Tip(l.deviceNaturalTip),
              ],
            ),
          ],
        ),
        SectionTitle(l.deviceKeys, note: later),
        if (keyboardUnknown) _Tip(l.deviceKeyboardUnknown),
        SfGroup(
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SfRow(
                  icon: Icons.keyboard_alt_outlined,
                  title: l.deviceModifiers,
                  subtitle: _presetNote(l, d.modifiers, differ: differ, localKeyboard: localKeyboard, name: name),
                  trailing: SfSegmented<ModifierPreset>(
                    label: l.deviceModifierPreset,
                    segments: [
                      (ModifierPreset.habit, _habitLabel(l, state.platform)),
                      (ModifierPreset.plain, l.deviceAsIs),
                      (ModifierPreset.custom, l.deviceCustom),
                    ],
                    value: d.modifiers,
                    onChanged: change(
                      (ModifierPreset v) => d.copyWith(
                        modifiers: v,
                        // 切到自定义时从当时的对应关系改起。
                        customModifiers: v == ModifierPreset.custom ? d.modifierMap(differ: differ) : null,
                      ),
                    ),
                  ),
                ),
                _ModifierTable(
                  settings: d,
                  differ: differ,
                  localMac: localMac,
                  peerMac: peerMac,
                  peerName: name,
                  onChanged: state.online
                      ? (key, target) => state.setDevice(
                          d.copyWith(
                            modifiers: ModifierPreset.custom,
                            customModifiers: d.modifierMap(differ: differ).replaced(key, target),
                          ),
                        )
                      : null,
                ),
              ],
            ),
            Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SfRow(
                  icon: Icons.compare_arrows_rounded,
                  title: l.deviceOtherKeys,
                  subtitle: l.deviceOtherKeysHint,
                  trailing: SmallButton(
                    label: l.keyMapAdd,
                    tone: PillTone.accent,
                    onPressed: state.online ? () => _addMapping(context, localMac: localMac, peerMac: peerMac) : null,
                  ),
                ),
                if (d.keys.isNotEmpty)
                  _Mappings(
                    settings: d,
                    localMac: localMac,
                    peerMac: peerMac,
                    onRemove: state.online
                        ? (m) => state.setDevice(
                            d.copyWith(
                              keys: [
                                for (final k in d.keys)
                                  if (k != m) k,
                              ],
                            ),
                          )
                        : null,
                  ),
              ],
            ),
            if (differ)
              SfRow(
                icon: localMac ? Icons.keyboard_command_key_rounded : Icons.keyboard_control_key_rounded,
                title: switch (state.platform) {
                  'macos' => l.deviceTextMac,
                  'linux' => l.deviceTextXkb,
                  'windows' => l.deviceTextWindows,
                  _ => l.deviceTextWindows,
                },
                subtitle: switch (localKeyboard) {
                  KeyboardKind.mac => l.deviceTextMacHint,
                  KeyboardKind.xkb => l.deviceTextXkbHint,
                  KeyboardKind.unknown => l.deviceTextWindowsHint,
                },
                trailing: SfSwitch(
                  value: d.textShortcuts,
                  label: l.deviceTextShortcuts,
                  onChanged: change((bool v) => d.copyWith(textShortcuts: v)),
                ),
              ),
          ],
        ),
        SectionTitle(l.deviceSwitching),
        SfGroup(
          children: [
            SfRow(
              icon: Icons.timer_outlined,
              title: l.deviceDwell,
              subtitle: server ? l.deviceDwellServerHint : l.deviceDwellClientHint,
              trailing: SfSegmented<int>(
                label: l.deviceDwellLabel,
                segments: [(0, l.deviceOff), (200, l.deviceSeconds('0.2')), (500, l.deviceSeconds('0.5'))],
                value: d.edgeDelayMs,
                onChanged: change((int v) => d.copyWith(edgeDelayMs: v)),
              ),
            ),
            SfRow(
              icon: Icons.content_paste_rounded,
              title: l.settingsClipboard,
              subtitle: state.settings.clipboard ? l.deviceClipboardHint : l.deviceClipboardOffHint,
              trailing: SfSwitch(
                value: d.clipboard && state.settings.clipboard,
                label: l.deviceClipboardLabel,
                onChanged: state.settings.clipboard ? change((bool v) => d.copyWith(clipboard: v)) : null,
              ),
            ),
            SfRow(
              icon: Icons.upload_file_outlined,
              title: l.deviceDropFiles,
              subtitle: l.deviceDropFilesHint,
              trailing: SfSwitch(
                value: d.files,
                label: l.deviceDropFiles,
                onChanged: change((bool v) => d.copyWith(files: v)),
              ),
            ),
          ],
        ),
        SectionTitle(l.deviceConnection),
        _Connection(peer: peer, server: state.settings.server),
        SectionTitle(l.deviceThisDevice),
        _Manage(peer: peer, settings: d),
      ],
    );
  }

  Future<void> _addMapping(BuildContext context, {required bool localMac, required bool peerMac}) async {
    final state = AppScope.read(context);
    final mapping = await showKeyMapDialog(context, peerName: peer.displayName, localMac: localMac, peerMac: peerMac);
    if (mapping == null) return;
    // 对话框开着的时候设置可能变了，按最新的改。同一个键再映射一次就替换旧的。
    final d = state.deviceSettings(peer.fingerprint);
    state.setDevice(
      d.copyWith(
        keys: [
          for (final k in d.keys)
            if (k.from != mapping.from) k,
          mapping,
        ],
      ),
    );
  }
}

String _habitLabel(AppLocalizations l, String platform) => switch (platform) {
  'macos' => l.deviceMacHabit,
  'linux' => l.deviceXkbHabit,
  'windows' => l.deviceWindowsHabit,
  _ => l.deviceAsIs,
};

/// 修饰键方案下面的一句说明。
String _presetNote(
  AppLocalizations l,
  ModifierPreset preset, {
  required bool differ,
  required KeyboardKind localKeyboard,
  required String name,
}) {
  if (localKeyboard == KeyboardKind.unknown) return l.deviceKeyboardUnknown;
  final localMac = localKeyboard == KeyboardKind.mac;
  return switch (preset) {
    ModifierPreset.habit when differ => localMac ? l.devicePresetMacDiffer(name) : l.devicePresetWinDiffer(name),
    ModifierPreset.habit => l.devicePresetSame,
    ModifierPreset.plain when differ => localMac ? l.devicePresetPlainMac : l.devicePresetPlainWin,
    ModifierPreset.plain => l.devicePresetPlain,
    ModifierPreset.custom => l.devicePresetCustom,
  };
}

// ── 头部与横幅 ───────────────────────────────

class _Header extends StatelessWidget {
  const _Header({required this.peer, required this.settings});

  final PeerInfo peer;
  final DeviceSettings settings;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final c = SfColors.of(context);
    final addr = state.peer?.fingerprint == peer.fingerprint ? state.peer?.addr ?? '' : '';
    final place = state.isServer ? state.placeOf(peer.fingerprint) : null;
    final renamed = peer.alias.trim().isNotEmpty;
    return SfCard(
      padding: const EdgeInsets.all(22),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          IconTile(state.peerIconOf(peer), size: 60, radius: 20, emphasized: peer.online),
          const SizedBox(width: 18),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                NameEditor(
                  name: peer.displayName,
                  label: context.l10n.deviceAlias,
                  placeholder: peer.name,
                  hint: context.l10n.deviceAliasHint,
                  enabled: state.online,
                  onSave: (v) => state.setDevice(settings.copyWith(alias: v == peer.name ? '' : v)),
                ),
                if (renamed)
                  Text(context.l10n.deviceOriginalName(peer.name), style: TextStyle(fontSize: 13, color: c.muted)),
                const SizedBox(height: 4),
                Text.rich(
                  TextSpan(
                    text: context.l10n.deviceSystemFingerprint(platformLabelOf(peer.platform)),
                    children: [
                      TextSpan(
                        text: peer.short,
                        style: const TextStyle(fontFamily: monoFont, fontFamilyFallback: monoFallback),
                      ),
                    ],
                  ),
                  style: TextStyle(fontSize: 14, color: c.muted),
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    Pill(
                      peer.online ? context.l10n.seenOnline : seenText(context.l10n, peer),
                      tone: peer.online ? PillTone.good : PillTone.plain,
                    ),
                    if (peer.online && addr.isNotEmpty)
                      Text(
                        addr,
                        style: TextStyle(fontSize: 13, color: c.muted, fontFeatures: tabular),
                      ),
                    if (!state.isServer && peer.online) Pill(context.l10n.deviceControlsThis, tone: PillTone.accent),
                    if (peer.version.isNotEmpty) Pill('ScreenFerry ${peer.version}'),
                    if (_newerThan(state, peer.version) case final target?)
                      Pill(context.l10n.devicePeerCanUpdate(target), tone: PillTone.accent),
                    if (place != null)
                      TextLink(
                        label: context.l10n.devicePlaceAdjust(place.side.name),
                        chevron: true,
                        onPressed: () => state.go(SfPage.layout),
                      ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 本机被控制时：键鼠方面的设置在对方那边改。
class _ControlledBanner extends StatelessWidget {
  const _ControlledBanner({required this.peer});

  final PeerInfo peer;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final c = SfColors.of(context);
    final name = peer.displayName;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 18),
      decoration: ShapeDecoration(color: c.accentSoft, shape: continuous(22)),
      child: BannerRow(
        leading: Icon(Icons.info_outline_rounded, size: 24, color: c.accent),
        body: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(context.l10n.deviceControlled, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700)),
            const SizedBox(height: 4),
            Text(context.l10n.deviceControlledHint(name), style: const TextStyle(fontSize: 14, height: 1.5)),
          ],
        ),
        trailing: PillButton(
          label: context.l10n.deviceOpenOn(name),
          icon: Icons.open_in_new_rounded,
          primary: true,
          onPressed: state.online && peer.online ? () => state.openPeerSettings(peer) : null,
        ),
      ),
    );
  }
}

/// 一组里的提示。
class _Tip extends StatelessWidget {
  const _Tip(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final c = SfColors.of(context);
    return Container(
      margin: const EdgeInsets.fromLTRB(14, 0, 14, 12),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      decoration: ShapeDecoration(color: c.accentSoft, shape: continuous(16)),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.info_outline_rounded, size: 18, color: c.accent),
          const SizedBox(width: 10),
          Expanded(child: Text(text, style: const TextStyle(fontSize: 14, height: 1.5))),
        ],
      ),
    );
  }
}

// ── 光标速度 ─────────────────────────────────

/// 光标速度：右上角可以填任意整数的 DPI；下面的拖动条按对数刻度、50 一档，本机的 DPI 在中间。
class _PointerSpeed extends StatefulWidget {
  const _PointerSpeed({required this.settings});

  final DeviceSettings settings;

  @override
  State<_PointerSpeed> createState() => _PointerSpeedState();
}

class _PointerSpeedState extends State<_PointerSpeed> {
  /// 拖动中的位置（0–1）。拖动时只改显示，松手才保存。
  double? _dragging;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final c = SfColors.of(context);
    final d = widget.settings;
    const range = DeviceSettings.pointerDpiRange;
    final base = state.settings.mouseDpi.clamp(range.min, range.max);
    // 拖动条的两端：本机的四分之一到四倍，不超出取值范围。
    final low = math.max(range.min, base ~/ 4);
    final high = math.min(range.max, base * 4);
    double positionOf(int dpi) => (math.log(dpi / low) / math.log(high / low)).clamp(0.0, 1.0);
    int dpiAt(double p) => ((low * math.pow(high / low, p)) / 50).round() * 50;

    final saved = d.pointerDpi == 0 ? base : d.pointerDpi;
    final dragging = _dragging;
    final shown = dragging == null ? saved : dpiAt(dragging).clamp(range.min, range.max);
    final changed = shown != base;
    // 和本机一样时记为 0：以后改了本机鼠标的 DPI，这台跟着变。
    void save(int dpi) {
      final v = dpi == base ? 0 : dpi;
      if (v != d.pointerDpi) state.setDevice(d.copyWith(pointerDpi: v));
    }

    final muted = TextStyle(fontSize: 12, color: c.muted, fontFeatures: tabular);
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 12, 20, 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const IconTile(Icons.mouse_outlined),
              const SizedBox(width: 14),
              Expanded(
                child: Text(
                  context.l10n.devicePointerSpeed,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                ),
              ),
              Text(
                '${(shown / base).toStringAsFixed(2)}×',
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w700,
                  fontFeatures: tabular,
                  color: changed ? c.accent : c.muted,
                ),
              ),
              const SizedBox(width: 10),
              DpiField(
                value: shown,
                label: context.l10n.devicePointerSpeed,
                min: range.min,
                max: range.max,
                highlight: changed,
                onChanged: state.online ? save : null,
              ),
            ],
          ),
          Padding(
            padding: const EdgeInsets.only(left: 54),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Slider(
                        value: dragging ?? positionOf(saved),
                        semanticFormatterCallback: (p) => '${dpiAt(p)} DPI',
                        onChanged: state.online ? (p) => setState(() => _dragging = p) : null,
                        onChangeEnd: state.online
                            ? (p) {
                                setState(() => _dragging = null);
                                save(dpiAt(p).clamp(range.min, range.max));
                              }
                            : null,
                      ),
                      SizedBox(
                        height: 16,
                        child: Stack(
                          children: [
                            Align(
                              alignment: Alignment.centerLeft,
                              child: Text('$low', style: muted),
                            ),
                            Align(
                              alignment: Alignment(positionOf(base) * 2 - 1, 0),
                              child: Text(context.l10n.deviceBaseHere(base), style: muted),
                            ),
                            Align(
                              alignment: Alignment.centerRight,
                              child: Text('$high', style: muted),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                SmallButton(
                  label: context.l10n.deviceReset,
                  onPressed: state.online && d.pointerDpi != 0 ? () => save(base) : null,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ── 按键 ─────────────────────────────────────

/// 四个修饰键的对照表：左边是本机的键，右边是对方收到的键（可以改）。被换掉的键蓝框。
class _ModifierTable extends StatelessWidget {
  const _ModifierTable({
    required this.settings,
    required this.differ,
    required this.localMac,
    required this.peerMac,
    required this.peerName,
    required this.onChanged,
  });

  final DeviceSettings settings;
  final bool differ;
  final bool localMac;
  final bool peerMac;
  final String peerName;
  final void Function(ModifierKey, ModifierTarget)? onChanged;

  static ModifierTarget _same(ModifierKey key) => switch (key) {
    ModifierKey.gui => ModifierTarget.gui,
    ModifierKey.ctrl => ModifierTarget.ctrl,
    ModifierKey.alt => ModifierTarget.alt,
    ModifierKey.capsLock => ModifierTarget.capsLock,
  };

  @override
  Widget build(BuildContext context) {
    final c = SfColors.of(context);
    final map = settings.modifierMap(differ: differ);
    final head = TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: c.muted);
    return Container(
      margin: const EdgeInsets.fromLTRB(20, 0, 20, 14),
      padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
      decoration: ShapeDecoration(color: c.card2, shape: continuous(18)),
      child: Column(
        children: [
          Row(
            children: [
              Expanded(child: Text(context.l10n.deviceLocalPress, style: head)),
              const SizedBox(width: 36),
              Expanded(
                child: Text(
                  context.l10n.devicePeerGets(peerName),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: head,
                ),
              ),
            ],
          ),
          for (final key in ModifierKey.values)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Row(
                children: [
                  Expanded(
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: KeyCap(modifierKeyName(key, mac: localMac)),
                    ),
                  ),
                  SizedBox(width: 36, child: Icon(Icons.arrow_forward_rounded, size: 18, color: c.muted)),
                  Expanded(
                    child: _TargetMenu(
                      value: map[key],
                      changed: map[key] != _same(key),
                      peerMac: peerMac,
                      label: context.l10n.deviceModifierBecomes(modifierKeyName(key, mac: localMac), peerName),
                      onSelected: onChanged == null ? null : (t) => onChanged!(key, t),
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

/// 选一个修饰键要变成的键。
class _TargetMenu extends StatelessWidget {
  const _TargetMenu({
    required this.value,
    required this.changed,
    required this.peerMac,
    required this.label,
    required this.onSelected,
  });

  final ModifierTarget value;
  final bool changed;
  final bool peerMac;
  final String label;
  final ValueChanged<ModifierTarget>? onSelected;

  @override
  Widget build(BuildContext context) {
    final c = SfColors.of(context);
    return PopupMenuButton<ModifierTarget>(
      tooltip: label,
      enabled: onSelected != null,
      initialValue: value,
      onSelected: onSelected,
      color: c.card,
      shape: continuous(14),
      itemBuilder: (_) => [
        for (final t in ModifierTarget.values)
          PopupMenuItem(
            value: t,
            child: Text(
              modifierTargetName(context.l10n, t, mac: peerMac),
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
          ),
      ],
      child: Container(
        height: 36,
        padding: const EdgeInsets.only(left: 12, right: 8),
        decoration: ShapeDecoration(
          color: c.card,
          shape: continuous(12, side: BorderSide(color: changed ? c.accent : c.line, width: 1.5)),
        ),
        child: Row(
          children: [
            Expanded(
              child: Text(
                modifierTargetName(context.l10n, value, mac: peerMac),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
              ),
            ),
            Icon(Icons.expand_more_rounded, size: 18, color: c.muted),
          ],
        ),
      ),
    );
  }
}

/// 其他按键映射的列表。
class _Mappings extends StatelessWidget {
  const _Mappings({required this.settings, required this.localMac, required this.peerMac, required this.onRemove});

  final DeviceSettings settings;
  final bool localMac;
  final bool peerMac;
  final ValueChanged<KeyMapping>? onRemove;

  @override
  Widget build(BuildContext context) {
    final c = SfColors.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 0, 20, 14),
      child: Column(
        children: [
          for (final m in settings.keys)
            Container(
              margin: const EdgeInsets.only(bottom: 8),
              padding: const EdgeInsets.fromLTRB(12, 6, 6, 6),
              decoration: ShapeDecoration(color: c.card2, shape: continuous(14)),
              child: Row(
                children: [
                  Expanded(
                    child: Wrap(
                      spacing: 10,
                      runSpacing: 6,
                      crossAxisAlignment: WrapCrossAlignment.center,
                      children: [
                        KeyCap(keyName(context.l10n, m.from, mac: localMac)),
                        Icon(Icons.arrow_forward_rounded, size: 18, color: c.muted),
                        if (m.to == 0)
                          Text(
                            context.l10n.keyMapNothing,
                            style: TextStyle(fontWeight: FontWeight.w600, color: c.muted),
                          )
                        else
                          KeyCap(keyName(context.l10n, m.to, mac: peerMac)),
                      ],
                    ),
                  ),
                  SfIconButton(
                    icon: Icons.close_rounded,
                    tooltip: context.l10n.deviceDeleteMapping,
                    size: 34,
                    onPressed: onRemove == null ? null : () => onRemove!(m),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

// ── 连接 ────────────────────────────────────

/// 上次连上的地址。本机被控制时选怎么找到控制端；本机控制别人时可以邀请它连过来。
class _Connection extends StatefulWidget {
  const _Connection({required this.peer, required this.server});

  final PeerInfo peer;

  /// 打开页面时的“固定地址”，空表示自动寻找。
  final String server;

  @override
  State<_Connection> createState() => _ConnectionState();
}

class _ConnectionState extends State<_Connection> {
  late final _controller = TextEditingController(
    text: widget.server.trim().isNotEmpty ? widget.server.trim() : widget.peer.addr,
  );

  /// 刚选了“固定地址”、还没填地址时为真。
  bool _choosing = false;

  @override
  void initState() {
    super.initState();
    _controller.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _choose(AppState state, bool fixed) {
    final text = _controller.text.trim();
    setState(() => _choosing = fixed && text.isEmpty);
    final next = fixed ? text : '';
    if (next != state.settings.server.trim() && (next.isNotEmpty || !fixed)) {
      state.updateSettings(SettingsPatch(server: next));
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final c = SfColors.of(context);
    final peer = widget.peer;
    final server = state.settings.server.trim();
    final fixed = server.isNotEmpty || _choosing;
    final text = _controller.text.trim();
    const mono = TextStyle(
      fontFamily: monoFont,
      fontFamilyFallback: monoFallback,
      fontSize: 15,
      fontWeight: FontWeight.w600,
    );
    return SfGroup(
      children: [
        SfRow(
          icon: Icons.history_rounded,
          title: context.l10n.deviceLastAddress,
          subtitle: state.isServer
              ? context.l10n.deviceLastAddressServerHint
              : context.l10n.deviceLastAddressClientHint,
          trailing: peer.addr.isEmpty
              ? Text(context.l10n.deviceNotYet, style: TextStyle(fontSize: 14, color: c.muted))
              : SelectableText(peer.addr, style: mono),
        ),
        if (state.isServer)
          SfRow(
            icon: Icons.login_rounded,
            title: context.l10n.deviceInvite,
            subtitle: peer.online ? context.l10n.deviceInviteOnlineHint : context.l10n.deviceInviteHint,
            trailing: SmallButton(
              label: context.l10n.deviceInviteButton,
              tone: PillTone.accent,
              onPressed: state.online && !peer.online
                  ? () => showAddressDialog(context, invite: true, initial: peer.addr)
                  : null,
            ),
          )
        else
          Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SfRow(
                icon: Icons.route_rounded,
                title: context.l10n.deviceConnectMode,
                subtitle: fixed ? context.l10n.deviceFixedHint : context.l10n.deviceAutoHint,
                trailing: SfSegmented<bool>(
                  label: context.l10n.deviceConnectMode,
                  segments: [(false, context.l10n.deviceAuto), (true, context.l10n.deviceFixed)],
                  value: fixed,
                  onChanged: state.online ? (v) => _choose(state, v) : null,
                ),
              ),
              if (fixed)
                Padding(
                  padding: const EdgeInsets.fromLTRB(14, 0, 14, 14),
                  child: Row(
                    children: [
                      Expanded(
                        child: Container(
                          height: 40,
                          padding: const EdgeInsets.symmetric(horizontal: 12),
                          decoration: ShapeDecoration(color: c.card2, shape: continuous(12)),
                          child: Semantics(
                            label: context.l10n.deviceServerAddress,
                            child: TextField(
                              controller: _controller,
                              enabled: state.online,
                              onSubmitted: (_) => _choose(state, true),
                              style: mono,
                              decoration: InputDecoration(
                                isDense: true,
                                border: InputBorder.none,
                                hintText: context.l10n.deviceAddressExample,
                                hintStyle: TextStyle(fontSize: 14, color: c.muted, fontWeight: FontWeight.w500),
                                contentPadding: const EdgeInsets.symmetric(vertical: 10),
                              ),
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      SmallButton(
                        label: context.l10n.commonSave,
                        tone: PillTone.accent,
                        onPressed: state.online && text.isNotEmpty && text != server
                            ? () => _choose(state, true)
                            : null,
                      ),
                    ],
                  ),
                ),
            ],
          ),
      ],
    );
  }
}

// ── 恢复默认与移除 ────────────────────────────

class _Manage extends StatefulWidget {
  const _Manage({required this.peer, required this.settings});

  final PeerInfo peer;
  final DeviceSettings settings;

  @override
  State<_Manage> createState() => _ManageState();
}

class _ManageState extends State<_Manage> {
  bool _confirming = false;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final d = widget.settings;
    return SfGroup(
      children: [
        SfRow(
          icon: Icons.restart_alt_rounded,
          title: context.l10n.deviceRestoreDefaults,
          subtitle: context.l10n.deviceRestoreDefaultsHint,
          trailing: SmallButton(
            label: context.l10n.deviceRestore,
            onPressed: state.online && d.reset() != d ? () => state.resetDevice(d.fingerprint) : null,
          ),
        ),
        SfRow(
          icon: Icons.link_off_rounded,
          title: context.l10n.deviceRemove,
          subtitle: _confirming ? context.l10n.deviceRemoveConfirm : context.l10n.deviceRemoveHint,
          trailing: AnimatedSwitcher(
            duration: Motion.fast,
            child: _confirming
                ? Row(
                    key: const ValueKey('confirm'),
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      SmallButton(
                        label: context.l10n.commonCancel,
                        onPressed: () => setState(() => _confirming = false),
                      ),
                      const SizedBox(width: 6),
                      SmallButton(
                        label: context.l10n.deviceRemoveButton,
                        danger: true,
                        onPressed: state.online ? () => state.forget(widget.peer) : null,
                      ),
                    ],
                  )
                : SmallButton(
                    key: const ValueKey('remove'),
                    label: context.l10n.deviceRemoveButton,
                    danger: true,
                    onPressed: state.online ? () => setState(() => _confirming = true) : null,
                  ),
          ),
        ),
      ],
    );
  }
}

/// 对方比本机知道的最新版本（本机的版本、或者发现的新版本）旧时，返回那个版本。
String? _newerThan(AppState state, String peerVersion) {
  if (peerVersion.isEmpty) return null;
  final latest = state.updateInfo.latest?.version;
  final target = latest != null && compareVersions(latest, state.version) > 0 ? latest : state.version;
  return target.isNotEmpty && compareVersions(target, peerVersion) > 0 ? target : null;
}
