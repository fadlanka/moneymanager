import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/app_theme.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';
import '../../widgets/month_pill.dart';
import '../../widgets/transaction_row.dart';
import 'transaction_detail_screen.dart';

class ActivityScreen extends ConsumerStatefulWidget {
  const ActivityScreen({super.key});

  @override
  ConsumerState<ActivityScreen> createState() => _ActivityScreenState();
}

class _ActivityScreenState extends ConsumerState<ActivityScreen> {
  final _search = TextEditingController();
  Timer? _debounce;

  @override
  void dispose() {
    _debounce?.cancel();
    _search.dispose();
    super.dispose();
  }

  void _onSearch(String v) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 350), () {
      final f = ref.read(txFilterProvider);
      ref.read(txFilterProvider.notifier).state = f.copyWith(search: v);
    });
  }

  @override
  Widget build(BuildContext context) {
    final filter = ref.watch(txFilterProvider);
    final txs = ref.watch(transactionsProvider);
    final checkCount = ref.watch(summaryProvider).valueOrNull?.needsCheckCount ?? 0;
    final chips = [
      ('all', S.all),
      ('out', S.moneyOut),
      ('in', S.moneyIn),
      ('check', checkCount > 0 ? '${S.toCheck} · $checkCount' : S.toCheck),
    ];

    return SafeArea(
      child: Column(children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
          child: Column(children: [
            const Row(children: [
              Expanded(
                child: Text(S.activity,
                    style: TextStyle(fontSize: 28, fontWeight: FontWeight.w800, letterSpacing: -0.5)),
              ),
              MonthPill(),
            ]),
            const SizedBox(height: 14),
            TextField(
              controller: _search,
              onChanged: _onSearch,
              decoration: const InputDecoration(
                hintText: S.searchHint,
                prefixIcon: Icon(Icons.search, color: AppColors.muted),
              ),
            ),
            const SizedBox(height: 14),
            SizedBox(
              height: 40,
              child: ListView(
                scrollDirection: Axis.horizontal,
                children: [
                  for (final (id, label) in chips) ...[
                    Pill(
                      label: label,
                      selected: filter.kind == id,
                      onTap: () => ref.read(txFilterProvider.notifier).state = filter.copyWith(kind: id),
                    ),
                    const SizedBox(width: 8),
                  ],
                ],
              ),
            ),
          ]),
        ),
        Expanded(
          child: RefreshIndicator(
            onRefresh: () async {
              refreshData(ref);
              await ref.read(transactionsProvider.future);
            },
            child: txs.when(
              loading: () => const Loading(),
              error: (e, _) => ListView(children: [ErrorView(error: e, onRetry: () => ref.invalidate(transactionsProvider))]),
              data: (list) => _List(list: list),
            ),
          ),
        ),
      ]),
    );
  }
}

class _List extends StatelessWidget {
  final List<Transaction> list;
  const _List({required this.list});

  @override
  Widget build(BuildContext context) {
    if (list.isEmpty) {
      return ListView(padding: const EdgeInsets.all(16), children: const [
        AppCard(
          padding: EdgeInsets.symmetric(vertical: 32, horizontal: 16),
          child: Text(S.noMatch, textAlign: TextAlign.center, style: TextStyle(color: AppColors.muted)),
        ),
      ]);
    }
    final net = list.fold<int>(0, (s, t) => s + (t.isIncome ? t.amount : -t.amount));
    // Group by local calendar day, newest first (the API already sorts).
    final groups = <String, List<Transaction>>{};
    for (final t in list) {
      groups.putIfAbsent(dayLabel(t.occurredAt), () => []).add(t);
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 6, 16, 28),
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          decoration: BoxDecoration(color: AppColors.ink, borderRadius: BorderRadius.circular(18)),
          child: Row(children: [
            Expanded(
              child: Text('${list.length} ${list.length == 1 ? 'transaction' : 'transactions'}',
                  style: const TextStyle(color: AppColors.mutedLight, fontWeight: FontWeight.w600)),
            ),
            Text('Net ${net >= 0 ? '+' : '−'}Rp ${groupThousands(net.abs())}',
                style: const TextStyle(
                    color: Colors.white, fontWeight: FontWeight.w800, fontSize: 17, fontFeatures: tabular)),
          ]),
        ),
        for (final entry in groups.entries) ...[
          const SizedBox(height: 18),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: Row(children: [
              Expanded(
                child: Text(entry.key,
                    style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
              ),
              Text(_dayTotal(entry.value),
                  style: const TextStyle(
                      fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted, fontFeatures: tabular)),
            ]),
          ),
          const SizedBox(height: 8),
          AppCard(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: Column(children: [
              for (var i = 0; i < entry.value.length; i++) ...[
                if (i > 0) const Divider(),
                TransactionRow(
                  tx: entry.value[i],
                  onTap: () => Navigator.push(
                    context,
                    MaterialPageRoute(builder: (_) => TransactionDetailScreen(tx: entry.value[i])),
                  ),
                ),
              ],
            ]),
          ),
        ],
      ],
    );
  }

  String _dayTotal(List<Transaction> txs) {
    final t = txs.fold<int>(0, (s, x) => s + (x.isIncome ? x.amount : -x.amount));
    return '${t >= 0 ? '+' : '−'}${groupThousands(t.abs())}';
  }
}
