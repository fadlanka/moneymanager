import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/app_theme.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';
import '../transactions/transaction_form_screen.dart';

class TransactionDetailScreen extends ConsumerWidget {
  final Transaction tx;
  const TransactionDetailScreen({super.key, required this.tx});

  Future<void> _delete(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text(S.deleteConfirm),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text(S.cancel)),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text(S.delete, style: TextStyle(color: AppColors.redText)),
          ),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;
    try {
      await ref.read(apiProvider).deleteTransaction(tx.id);
      refreshData(ref);
      if (context.mounted) Navigator.pop(context);
    } on ApiException catch (e) {
      if (context.mounted) showSnack(context, e.message);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cat = tx.categoryId == null ? null : ref.watch(categoryByIdProvider)[tx.categoryId];
    final source = switch (tx.source) {
      'scan' => 'Scanned receipt',
      'debt_payment' => 'Debt payment',
      _ => 'Added by hand',
    };

    Widget row(String label, String value) => Padding(
          padding: const EdgeInsets.symmetric(vertical: 12),
          child: Row(children: [
            Text(label, style: const TextStyle(color: AppColors.muted)),
            const SizedBox(width: 16),
            Expanded(child: Text(value, textAlign: TextAlign.end, style: const TextStyle(fontWeight: FontWeight.w600))),
          ]),
        );

    return Scaffold(
      appBar: AppBar(title: const Text(S.transaction, style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700))),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
        children: [
          Center(child: CategoryBadge(slug: cat?.slug ?? (tx.isIncome ? 'income' : 'other'), color: cat?.color, size: 64)),
          const SizedBox(height: 8),
          Text(tx.merchant, textAlign: TextAlign.center, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          FittedBox(
            fit: BoxFit.scaleDown,
            child: Text(
              '${tx.isIncome ? '+' : '−'}Rp ${groupThousands(tx.amount)}',
              style: TextStyle(
                fontSize: 40,
                fontWeight: FontWeight.w800,
                letterSpacing: -1,
                color: tx.isIncome ? AppColors.greenText : AppColors.ink,
                fontFeatures: tabular,
              ),
            ),
          ),
          const SizedBox(height: 8),
          Wrap(alignment: WrapAlignment.center, spacing: 6, children: [
            if (tx.fromScan) const Tag(S.aiScan, tone: TagTone.blue),
            if (tx.needsCheck) const Tag(S.check, tone: TagTone.yellow),
          ]),
          const SizedBox(height: 16),
          AppCard(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: Column(children: [
              row(S.category, cat?.name ?? 'Uncategorized'),
              const Divider(),
              row(S.date, dateTime(tx.occurredAt)),
              const Divider(),
              row(S.source, source),
              if (tx.note != null) ...[const Divider(), row('Note', tx.note!)],
            ]),
          ),
          if (tx.items.isNotEmpty) ...[
            const SizedBox(height: 16),
            AppCard(
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
              child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
                const SectionHeader(S.items),
                for (final it in tx.items)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Row(children: [
                      Expanded(
                        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                          Text(it.name, style: const TextStyle(fontWeight: FontWeight.w600)),
                          if (it.unitPrice != null)
                            Text('${_qty(it.qty)} × ${rupiah(it.unitPrice!)}',
                                style: const TextStyle(fontSize: 13, color: AppColors.muted)),
                        ]),
                      ),
                      Amount(groupThousands(it.total), size: 15),
                    ]),
                  ),
              ]),
            ),
          ],
          const SizedBox(height: 16),
          Row(children: [
            Expanded(
              child: OutlinedButton(
                onPressed: () async {
                  final changed = await Navigator.push<bool>(
                    context,
                    MaterialPageRoute(builder: (_) => TransactionFormScreen(existing: tx)),
                  );
                  if (changed == true && context.mounted) Navigator.pop(context);
                },
                child: const Text(S.edit),
              ),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: OutlinedButton(
                onPressed: () => _delete(context, ref),
                style: OutlinedButton.styleFrom(foregroundColor: AppColors.redText),
                child: const Text(S.delete),
              ),
            ),
          ]),
        ],
      ),
    );
  }
}

String _qty(double q) => q == q.roundToDouble() ? q.toInt().toString() : q.toString();
