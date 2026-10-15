import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:receive_sharing_intent/receive_sharing_intent.dart';

import '../../l10n/strings.dart';
import '../../theme/tokens.dart';
import '../activity/activity_screen.dart';
import '../budget/budget_screen.dart';
import '../debts/debts_screen.dart';
import '../home/home_screen.dart';
import '../scan/processing_screen.dart';
import '../scan/scan_screen.dart';

enum ShellTab { home, activity, budget, debts }

final shellTabProvider = StateProvider<ShellTab>((ref) => ShellTab.home);

class AppShell extends ConsumerStatefulWidget {
  const AppShell({super.key});

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  StreamSubscription<List<SharedMediaFile>>? _shareSub;

  @override
  void initState() {
    super.initState();
    // Android/iOS share sheet. Not available in the web (PWA) build.
    if (!kIsWeb) {
      _shareSub = ReceiveSharingIntent.instance.getMediaStream().listen(_onShared);
      ReceiveSharingIntent.instance.getInitialMedia().then((files) {
        _onShared(files);
        ReceiveSharingIntent.instance.reset();
      });
    }
  }

  @override
  void dispose() {
    _shareSub?.cancel();
    super.dispose();
  }

  Future<void> _onShared(List<SharedMediaFile> files) async {
    final image = files.where((f) => f.type == SharedMediaType.image).firstOrNull;
    if (image == null || !mounted) return;
    final file = XFile(image.path);
    final bytes = await file.readAsBytes();
    if (!mounted) return;
    // Straight to "reading receipt", skipping the camera step.
    await Navigator.of(context).push(MaterialPageRoute(
      builder: (_) => ProcessingScreen(bytes: bytes, filename: file.name),
    ));
  }

  void _openScan() => Navigator.of(context).push(MaterialPageRoute(builder: (_) => const ScanScreen()));

  @override
  Widget build(BuildContext context) {
    final tab = ref.watch(shellTabProvider);
    return Scaffold(
      body: IndexedStack(
        index: tab.index,
        children: [
          HomeScreen(onScan: _openScan),
          const ActivityScreen(),
          const BudgetScreen(),
          const DebtsScreen(),
        ],
      ),
      bottomNavigationBar: _TabBar(
        current: tab,
        onSelect: (t) => ref.read(shellTabProvider.notifier).state = t,
        onScan: _openScan,
      ),
    );
  }
}

/// Bottom bar from the design: four tabs and a raised Scan button in the middle.
class _TabBar extends StatelessWidget {
  final ShellTab current;
  final ValueChanged<ShellTab> onSelect;
  final VoidCallback onScan;
  const _TabBar({required this.current, required this.onSelect, required this.onScan});

  @override
  Widget build(BuildContext context) {
    Widget item(ShellTab t, IconData icon, String label) {
      final on = t == current;
      final color = on ? AppColors.blue : AppColors.muted;
      return Expanded(
        child: Semantics(
          button: true,
          selected: on,
          label: label,
          child: InkWell(
            onTap: () => onSelect(t),
            child: SizedBox(
              height: 56,
              child: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
                Icon(icon, color: color, size: 24),
                const SizedBox(height: 4),
                Text(label,
                    style: TextStyle(fontSize: 12, color: color, fontWeight: on ? FontWeight.w700 : FontWeight.w600)),
              ]),
            ),
          ),
        ),
      );
    }

    return Container(
      decoration: const BoxDecoration(
        color: AppColors.card,
        border: Border(top: BorderSide(color: AppColors.hair)),
      ),
      child: SafeArea(
        top: false,
        child: SizedBox(
          height: 64,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              item(ShellTab.home, Icons.home_outlined, S.home),
              item(ShellTab.activity, Icons.receipt_outlined, S.activity),
              Expanded(
                child: Center(
                  child: Transform.translate(
                    offset: const Offset(0, -16),
                    child: Semantics(
                      button: true,
                      label: S.scanReceipt,
                      child: Material(
                        color: AppColors.blue,
                        shape: const CircleBorder(side: BorderSide(color: AppColors.bg, width: 5)),
                        elevation: 0,
                        child: InkWell(
                          customBorder: const CircleBorder(),
                          onTap: onScan,
                          child: const SizedBox(
                            width: 62,
                            height: 62,
                            child: Icon(Icons.document_scanner_outlined, color: Colors.white, size: 26),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              item(ShellTab.budget, Icons.pie_chart_outline, S.budget),
              item(ShellTab.debts, Icons.credit_card_outlined, S.debts),
            ],
          ),
        ),
      ),
    );
  }
}
