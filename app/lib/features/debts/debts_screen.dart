import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';
import 'debt_form_screen.dart';

final _strategyProvider = StateProvider<String>((ref) => 'avalanche');
final _extraProvider = StateProvider<int>((ref) => 0);

final _planProvider = FutureProvider<DebtPlan>((ref) async {
  ref.watch(debtsProvider);
  final strategy = ref.watch(_strategyProvider);
  final extra = ref.watch(_extraProvider);
  return ref.read(apiProvider).debtPlan(strategy, extra);
});

class DebtsScreen extends ConsumerWidget {
  const DebtsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final debts = ref.watch(debtsProvider);
    return SafeArea(
      child: RefreshIndicator(
        onRefresh: () async {
          ref.invalidate(debtsProvider);
          await ref.read(debtsProvider.future);
        },
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 20, 16, 28),
          children: [
            Row(children: [
              const Expanded(
                child: Text(S.debts, style: TextStyle(fontSize: 28, fontWeight: FontWeight.w800, letterSpacing: -0.5)),
              ),
              FilledButton.icon(
                style: FilledButton.styleFrom(minimumSize: const Size(0, AppSizes.buttonSm)),
                onPressed: () => Navigator.push(context, MaterialPageRoute(builder: (_) => const DebtFormScreen())),
                icon: const Icon(Icons.add, size: 18),
                label: const Text(S.newDebt),
              ),
            ]),
            const SizedBox(height: 18),
            debts.when(
              loading: () => const Loading(),
              error: (e, _) => ErrorView(error: e, onRetry: () => ref.invalidate(debtsProvider)),
              data: (list) => list.isEmpty
                  ? const AppCard(
                      padding: EdgeInsets.symmetric(vertical: 32, horizontal: 16),
                      child: Text(S.noDebts, textAlign: TextAlign.center, style: TextStyle(color: AppColors.muted)),
                    )
                  : _DebtList(list: list),
            ),
          ],
        ),
      ),
    );
  }
}

class _DebtList extends ConsumerWidget {
  final List<Debt> list;
  const _DebtList({required this.list});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final open = list.where((d) => !d.settled).toList();
    final remaining = open.fold<int>(0, (s, d) => s + d.balance);
    final monthly = open.fold<int>(0, (s, d) => s + d.monthlyPayment);
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      Row(children: [
        Expanded(child: _Stat(label: 'Still to pay', value: rupiah(remaining), bg: AppColors.redSoft, fg: AppColors.redText)),
        const SizedBox(width: 10),
        Expanded(child: _Stat(label: 'Per month', value: rupiah(monthly), bg: AppColors.blueSoft, fg: AppColors.blue)),
      ]),
      const SizedBox(height: 16),
      for (final d in list) ...[_DebtCard(debt: d), const SizedBox(height: 12)],
      if (open.isNotEmpty) ...[const SizedBox(height: 8), const _PlanCard()],
    ]);
  }
}

class _Stat extends StatelessWidget {
  final String label;
  final String value;
  final Color bg;
  final Color fg;
  const _Stat({required this.label, required this.value, required this.bg, required this.fg});

  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(20)),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(label, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: fg)),
          const SizedBox(height: 6),
          FittedBox(fit: BoxFit.scaleDown, child: Amount(value, size: 20, weight: FontWeight.w800)),
        ]),
      );
}

class _DebtCard extends ConsumerWidget {
  final Debt debt;
  const _DebtCard({required this.debt});

  Future<void> _pay(BuildContext context, WidgetRef ref) async {
    final suggested = debt.monthlyPayment > 0 && debt.monthlyPayment < debt.balance ? debt.monthlyPayment : debt.balance;
    final amount = await showModalBottomSheet<int>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => _PaySheet(name: debt.name, suggested: suggested),
    );
    if (amount == null || amount <= 0) return;
    try {
      await ref.read(apiProvider).payDebt(debt.id, amount);
      refreshData(ref);
      if (context.mounted) showSnack(context, S.saved);
    } on ApiException catch (e) {
      if (context.mounted) showSnack(context, e.message);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final now = DateTime.now();
    final paidUp = debt.monthlyPayment > 0 && debt.paidThisMonth >= debt.monthlyPayment;
    final Widget status;
    if (debt.settled) {
      status = const Tag('Paid off', tone: TagTone.green);
    } else if (paidUp) {
      status = const Tag(S.paidThisMonth, tone: TagTone.green);
    } else if (debt.dueDay != null) {
      final lastDay = DateTime(now.year, now.month + 1, 0).day;
      final day = debt.dueDay! > lastDay ? lastDay : debt.dueDay!;
      final overdue = now.day > day;
      status = Tag(overdue ? 'Overdue · ${shortDate(DateTime(now.year, now.month, day))}' : 'Due ${shortDate(DateTime(now.year, now.month, day))}',
          tone: overdue ? TagTone.red : TagTone.yellow);
    } else {
      status = const Tag('No due date');
    }

    return AppCard(
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Row(children: [
          const CategoryBadge(slug: 'debt', color: 'grey', size: 40),
          const SizedBox(width: 12),
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(debt.name, style: const TextStyle(fontWeight: FontWeight.w700)),
              Text(
                [
                  debt.kind == 'loan' ? S.loan : S.installment,
                  if (debt.creditor != null) debt.creditor!,
                  if (debt.interestRateBp > 0) '${(debt.interestRateBp / 100).toStringAsFixed(1)}%/yr',
                ].join(' · '),
                style: const TextStyle(fontSize: 13, color: AppColors.muted),
              ),
            ]),
          ),
          IconButton(
            tooltip: S.edit,
            icon: const Icon(Icons.edit_outlined, size: 20),
            onPressed: () => Navigator.push(context, MaterialPageRoute(builder: (_) => DebtFormScreen(existing: debt))),
          ),
        ]),
        const SizedBox(height: 12),
        ProgressBar(value: debt.progress, color: AppColors.green, semanticsLabel: '${debt.name} paid off'),
        const SizedBox(height: 8),
        Row(children: [
          Expanded(
            child: Text('${rupiah(debt.balance)} left of ${groupThousands(debt.principal)}',
                style: const TextStyle(fontSize: 13, color: AppColors.muted)),
          ),
          status,
        ]),
        if (!debt.settled) ...[
          const SizedBox(height: 12),
          Row(children: [
            Expanded(
              child: Text(
                debt.monthlyPayment > 0 ? '${rupiah(debt.monthlyPayment)} / month' : 'No fixed payment',
                style: const TextStyle(fontWeight: FontWeight.w600),
              ),
            ),
            FilledButton(
              style: FilledButton.styleFrom(minimumSize: const Size(88, AppSizes.buttonSm)),
              onPressed: () => _pay(context, ref),
              child: const Text(S.pay),
            ),
          ]),
        ],
      ]),
    );
  }
}

class _PlanCard extends ConsumerStatefulWidget {
  const _PlanCard();

  @override
  ConsumerState<_PlanCard> createState() => _PlanCardState();
}

class _PlanCardState extends ConsumerState<_PlanCard> {
  final _extra = TextEditingController();

  @override
  void dispose() {
    _extra.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final strategy = ref.watch(_strategyProvider);
    final plan = ref.watch(_planProvider);
    return AppCard(
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        const SectionHeader(S.payoffPlan),
        const SizedBox(height: 12),
        Segmented<String>(
          options: const [('avalanche', S.avalanche), ('snowball', S.snowball)],
          value: strategy,
          onChanged: (v) => ref.read(_strategyProvider.notifier).state = v,
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _extra,
          keyboardType: TextInputType.number,
          inputFormatters: [RupiahInputFormatter()],
          decoration: const InputDecoration(labelText: S.extraPerMonth, prefixText: 'Rp '),
          onSubmitted: (v) => ref.read(_extraProvider.notifier).state = parseRupiah(v) ?? 0,
          onTapOutside: (_) {
            ref.read(_extraProvider.notifier).state = parseRupiah(_extra.text) ?? 0;
            FocusScope.of(context).unfocus();
          },
        ),
        const SizedBox(height: 16),
        plan.when(
          loading: () => const Loading(),
          error: (e, _) => Text(e.toString(), style: const TextStyle(color: AppColors.redText)),
          data: (p) {
            if (!p.feasible) {
              return const Notice(
                text: 'At these payments some debts never finish: interest grows faster than you pay. '
                    'Add an extra amount per month.',
                bg: AppColors.redSoft,
                iconBg: AppColors.red,
              );
            }
            return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              Text(
                'Debt-free by ${p.finishDate == null ? '–' : monthYear(p.finishDate!)}'
                ' · ${p.months} ${p.months == 1 ? 'month' : 'months'}',
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 4),
              Text('Paying ${rupiah(p.monthlyBudget)} a month · interest ${rupiah(p.totalInterest)}',
                  style: const TextStyle(fontSize: 13, color: AppColors.muted)),
              const SizedBox(height: 12),
              for (final d in p.debts)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 6),
                  child: Row(children: [
                    CircleAvatar(
                      radius: 12,
                      backgroundColor: AppColors.track,
                      child: Text('${d.priority}', style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w700)),
                    ),
                    const SizedBox(width: 10),
                    Expanded(child: Text(d.name, style: const TextStyle(fontWeight: FontWeight.w600))),
                    Text(
                      p.payoffDates[d.id] == null ? '–' : monthYear(p.payoffDates[d.id]!),
                      style: const TextStyle(color: AppColors.muted),
                    ),
                  ]),
                ),
            ]);
          },
        ),
      ]),
    );
  }
}

class _PaySheet extends StatefulWidget {
  final String name;
  final int suggested;
  const _PaySheet({required this.name, required this.suggested});

  @override
  State<_PaySheet> createState() => _PaySheetState();
}

class _PaySheetState extends State<_PaySheet> {
  late final TextEditingController _amount = TextEditingController(text: groupThousands(widget.suggested));

  @override
  void dispose() {
    _amount.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Padding(
        padding: EdgeInsets.fromLTRB(16, 0, 16, 16 + MediaQuery.of(context).viewInsets.bottom),
        child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          Text('Pay ${widget.name}', style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          const SizedBox(height: 4),
          const Text('Recorded as an expense in Debt & installments.', style: TextStyle(color: AppColors.muted)),
          const SizedBox(height: 16),
          TextField(
            controller: _amount,
            autofocus: true,
            keyboardType: TextInputType.number,
            inputFormatters: [RupiahInputFormatter()],
            decoration: const InputDecoration(labelText: S.amount, prefixText: 'Rp '),
          ),
          const SizedBox(height: 16),
          FilledButton(
            onPressed: () => Navigator.pop(context, parseRupiah(_amount.text)),
            child: const Text(S.pay),
          ),
        ]),
      );
}
