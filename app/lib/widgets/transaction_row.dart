import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models.dart';
import '../l10n/strings.dart';
import '../state/providers.dart';
import '../theme/tokens.dart';
import '../util/format.dart';
import 'common.dart';

/// One transaction line: category badge, store, subtitle with tags, signed amount.
class TransactionRow extends ConsumerWidget {
  final Transaction tx;
  final VoidCallback? onTap;
  /// Show "Category · Today 19:05" instead of "Category · 19:05".
  final bool subtitleTime;
  const TransactionRow({super.key, required this.tx, this.onTap, this.subtitleTime = false});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cat = tx.categoryId == null ? null : ref.watch(categoryByIdProvider)[tx.categoryId];
    final when = subtitleTime ? '${dayLabel(tx.occurredAt)} ${timeOnly(tx.occurredAt)}' : timeOnly(tx.occurredAt);
    final sub = '${cat?.name ?? (tx.isIncome ? S.income : 'Uncategorized')} · $when';
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 12),
        child: Row(children: [
          CategoryBadge(slug: cat?.slug ?? (tx.isIncome ? 'income' : 'other'), color: cat?.color ?? 'grey'),
          const SizedBox(width: 12),
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(tx.merchant,
                  maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontWeight: FontWeight.w700)),
              const SizedBox(height: 2),
              Wrap(spacing: 6, runSpacing: 4, crossAxisAlignment: WrapCrossAlignment.center, children: [
                Text(sub, style: const TextStyle(fontSize: 13, color: AppColors.muted)),
                if (tx.fromScan) const Tag(S.aiScan, tone: TagTone.blue),
                if (tx.needsCheck) const Tag(S.check, tone: TagTone.yellow),
              ]),
            ]),
          ),
          const SizedBox(width: 8),
          Amount(
            signedAmount(tx.amount, income: tx.isIncome),
            color: tx.isIncome ? AppColors.greenText : AppColors.ink,
          ),
        ]),
      ),
    );
  }
}
