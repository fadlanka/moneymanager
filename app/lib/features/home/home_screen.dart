import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/app_theme.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';
import '../../widgets/transaction_row.dart';
import '../activity/transaction_detail_screen.dart';
import '../budget/budget_edit_screen.dart';
import '../budget/budget_screen.dart';
import '../profile/profile_screen.dart';
import '../shell/app_shell.dart';
import '../transactions/transaction_form_screen.dart';

class HomeScreen extends ConsumerWidget {
  final VoidCallback onScan;
  const HomeScreen({super.key, required this.onScan});

  String _greeting() {
    final h = DateTime.now().hour;
    if (h < 11) return S.goodMorning;
    if (h < 17) return S.goodAfternoon;
    return S.goodEvening;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final user = ref.watch(authProvider).user!;
    final summary = ref.watch(summaryProvider);
    return SafeArea(
      child: RefreshIndicator(
        onRefresh: () async {
          refreshData(ref);
          await ref.read(summaryProvider.future);
        },
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 20, 16, 28),
          children: [
            Row(children: [
              Semantics(
                button: true,
                label: S.profile,
                child: GestureDetector(
                  onTap: () => Navigator.push(context, MaterialPageRoute(builder: (_) => const ProfileScreen())),
                  child: CircleAvatar(
                    radius: 22,
                    backgroundColor: AppColors.blue,
                    child: Text(user.initials,
                        style: const TextStyle(color: Colors.white, fontWeight: FontWeight.w700, fontSize: 14)),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(_greeting(),
                      style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
                  Text(user.name.split(' ').first,
                      style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w800)),
                ]),
              ),
            ]),
            const SizedBox(height: 20),
            summary.when(
              loading: () => const Loading(),
              error: (e, _) => ErrorView(error: e, onRetry: () => ref.invalidate(summaryProvider)),
              data: (s) => _Body(summary: s, onScan: onScan),
            ),
          ],
        ),
      ),
    );
  }
}

class _Body extends ConsumerWidget {
  final Summary summary;
  final VoidCallback onScan;
  const _Body({required this.summary, required this.onScan});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = summary;
    final over = s.budget.categories.where((c) => c.isOver).toList();
    final meters = s.budget.budgeted.take(3).toList();

    void goTab(ShellTab t) => ref.read(shellTabProvider.notifier).state = t;

    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      _BalanceCard(summary: s),
      const SizedBox(height: 20),
      Row(children: [
        _QuickAction(icon: Icons.document_scanner_outlined, label: S.scan, primary: true, onTap: onScan),
        _QuickAction(
          icon: Icons.add,
          label: S.add,
          onTap: () => Navigator.push(context, MaterialPageRoute(builder: (_) => const TransactionFormScreen())),
        ),
        _QuickAction(icon: Icons.credit_card_outlined, label: S.debts, onTap: () => goTab(ShellTab.debts)),
        _QuickAction(
          icon: Icons.insights_outlined,
          label: S.insights,
          onTap: () {
            ref.read(budgetSectionProvider.notifier).state = BudgetSection.insights;
            goTab(ShellTab.budget);
          },
        ),
      ]),
      if (over.isNotEmpty) ...[
        const SizedBox(height: 20),
        Notice(
          text: over.length == 1
              ? '${over.first.name} is ${rupiah(over.first.spent - (over.first.limit ?? 0))} over budget.'
              : '${over.length} categories are over budget.',
          icon: Icons.warning_amber_rounded,
          trailing: const Text(S.review, style: TextStyle(fontWeight: FontWeight.w700)),
          onTap: () {
            ref.read(budgetSectionProvider.notifier).state = BudgetSection.categories;
            goTab(ShellTab.budget);
          },
        ),
      ],
      const SizedBox(height: 20),
      AppCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          SectionHeader(S.budgets, action: S.seeAll, onAction: () {
            ref.read(budgetSectionProvider.notifier).state = BudgetSection.categories;
            goTab(ShellTab.budget);
          }),
          if (meters.isEmpty) ...[
            const SizedBox(height: 8),
            const Text(S.setUpBudget, style: TextStyle(color: AppColors.muted)),
            const SizedBox(height: 12),
            OutlinedButton(
              onPressed: () => Navigator.push(context, MaterialPageRoute(builder: (_) => const BudgetEditScreen())),
              child: const Text(S.setUpBudgetCta),
            ),
          ],
          for (var i = 0; i < meters.length; i++) ...[
            if (i > 0) const Divider(),
            _Meter(c: meters[i]),
          ],
        ]),
      ),
      const SizedBox(height: 20),
      AppCard(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          SectionHeader(S.recent, action: S.seeAll, onAction: () => goTab(ShellTab.activity)),
          if (s.recent.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16),
              child: Text(S.noTransactions, style: TextStyle(color: AppColors.muted)),
            ),
          for (var i = 0; i < s.recent.length; i++) ...[
            if (i > 0) const Divider(),
            TransactionRow(
              tx: s.recent[i],
              subtitleTime: true,
              onTap: () => Navigator.push(
                context,
                MaterialPageRoute(builder: (_) => TransactionDetailScreen(tx: s.recent[i])),
              ),
            ),
          ],
        ]),
      ),
    ]);
  }
}

class _BalanceCard extends StatelessWidget {
  final Summary summary;
  const _BalanceCard({required this.summary});

  @override
  Widget build(BuildContext context) {
    Widget chip(IconData icon, Color iconBg, String label, int value) => Expanded(
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(color: const Color(0x1FFFFFFF), borderRadius: BorderRadius.circular(16)),
            child: Row(children: [
              Container(
                width: 30,
                height: 30,
                decoration: BoxDecoration(color: iconBg, shape: BoxShape.circle),
                child: Icon(icon, size: 16, color: AppColors.ink),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(label, style: const TextStyle(fontSize: 12, color: AppColors.blueOnDark)),
                  FittedBox(
                    fit: BoxFit.scaleDown,
                    alignment: Alignment.centerLeft,
                    child: Amount(groupThousands(value), size: 15, color: Colors.white),
                  ),
                ]),
              ),
            ]),
          ),
        );

    return Container(
      padding: const EdgeInsets.fromLTRB(20, 22, 20, 20),
      decoration: BoxDecoration(color: AppColors.blue, borderRadius: BorderRadius.circular(24)),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text('Net · ${monthName(summary.month)}',
            style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: AppColors.blueOnDark)),
        const SizedBox(height: 4),
        FittedBox(
          fit: BoxFit.scaleDown,
          alignment: Alignment.centerLeft,
          child: Text(
            rupiah(summary.net),
            style: const TextStyle(
              fontSize: 34,
              fontWeight: FontWeight.w800,
              letterSpacing: -1,
              color: Colors.white,
              fontFeatures: tabular,
            ),
          ),
        ),
        const SizedBox(height: 18),
        Row(children: [
          chip(Icons.arrow_downward_rounded, AppColors.green, S.moneyIn, summary.income),
          const SizedBox(width: 10),
          chip(Icons.arrow_upward_rounded, AppColors.red, S.moneyOut, summary.expense),
        ]),
      ]),
    );
  }
}

class _QuickAction extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool primary;
  final VoidCallback onTap;
  const _QuickAction({required this.icon, required this.label, required this.onTap, this.primary = false});

  @override
  Widget build(BuildContext context) => Expanded(
        child: Semantics(
          button: true,
          label: label,
          child: InkWell(
            borderRadius: BorderRadius.circular(18),
            onTap: onTap,
            child: Column(children: [
              Container(
                width: 56,
                height: 56,
                decoration: BoxDecoration(
                  color: primary ? AppColors.blue : AppColors.card,
                  borderRadius: BorderRadius.circular(18),
                  border: primary ? null : Border.all(color: AppColors.hair),
                ),
                child: Icon(icon, color: primary ? Colors.white : AppColors.ink),
              ),
              const SizedBox(height: 8),
              Text(label, style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
            ]),
          ),
        ),
      );
}

class _Meter extends StatelessWidget {
  final CategoryStatus c;
  const _Meter({required this.c});

  @override
  Widget build(BuildContext context) {
    final pct = c.pct ?? 0;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 12),
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Row(children: [
          CategoryBadge(slug: c.slug, color: c.color, size: 36),
          const SizedBox(width: 10),
          Expanded(child: Text(c.name, style: const TextStyle(fontWeight: FontWeight.w600))),
          Text(c.isOver ? 'Over · $pct%' : '$pct%',
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: c.isOver ? AppColors.redText : AppColors.ink,
                fontFeatures: tabular,
              )),
        ]),
        const SizedBox(height: 8),
        ProgressBar(value: pct / 100, color: budgetColor(c.state), semanticsLabel: c.name),
        const SizedBox(height: 8),
        Text('${rupiah(c.spent)} of ${rupiah(c.limit ?? 0)}',
            style: const TextStyle(fontSize: 13, color: AppColors.muted, fontFeatures: tabular)),
      ]),
    );
  }
}
