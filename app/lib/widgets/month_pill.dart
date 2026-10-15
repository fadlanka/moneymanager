import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../state/providers.dart';
import '../theme/tokens.dart';
import '../util/format.dart';
import 'common.dart';

/// "Oct 2026 ▾" pill that changes the month shared by Home, Activity and Budget.
class MonthPill extends ConsumerWidget {
  const MonthPill({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final month = ref.watch(selectedMonthProvider);
    return Pill(
      label: monthYear(month),
      trailing: const Icon(Icons.expand_more, size: 18, color: AppColors.ink),
      onTap: () async {
        final picked = await showModalBottomSheet<DateTime>(
          context: context,
          showDragHandle: true,
          builder: (_) => _MonthSheet(selected: month),
        );
        if (picked != null) ref.read(selectedMonthProvider.notifier).state = picked;
      },
    );
  }
}

class _MonthSheet extends StatelessWidget {
  final DateTime selected;
  const _MonthSheet({required this.selected});

  @override
  Widget build(BuildContext context) {
    final now = firstOfMonth(DateTime.now());
    final months = [for (var i = 0; i < 12; i++) DateTime(now.year, now.month - i)];
    return SafeArea(
      child: ListView(
        shrinkWrap: true,
        children: [
          for (final m in months)
            ListTile(
              title: Text(monthYear(m), style: const TextStyle(fontWeight: FontWeight.w600)),
              trailing: m == selected ? const Icon(Icons.check, color: AppColors.blue) : null,
              onTap: () => Navigator.pop(context, m),
            ),
        ],
      ),
    );
  }
}
