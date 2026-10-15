import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/app_theme.dart';
import '../../theme/category_style.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';
import '../../widgets/month_pill.dart';
import 'budget_edit_screen.dart';

enum BudgetSection { overview, categories, insights }

final budgetSectionProvider = StateProvider<BudgetSection>((ref) => BudgetSection.overview);

/// Months where the user tapped "Not now" on the suggestion (this session only).
final _dismissedSuggestionProvider = StateProvider<Set<DateTime>>((ref) => <DateTime>{});

class BudgetScreen extends ConsumerWidget {
  const BudgetScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final section = ref.watch(budgetSectionProvider);
    final summary = ref.watch(summaryProvider);
    return SafeArea(
      child: Column(children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 20, 16, 10),
          child: Column(children: [
            const Row(children: [
              Expanded(
                child: Text(S.budget, style: TextStyle(fontSize: 28, fontWeight: FontWeight.w800, letterSpacing: -0.5)),
              ),
              MonthPill(),
            ]),
            const SizedBox(height: 14),
            Segmented<BudgetSection>(
              options: const [
                (BudgetSection.overview, S.overview),
                (BudgetSection.categories, S.categories),
                (BudgetSection.insights, S.insights),
              ],
              value: section,
              onChanged: (v) => ref.read(budgetSectionProvider.notifier).state = v,
            ),
          ]),
        ),
        Expanded(
          child: RefreshIndicator(
            onRefresh: () async {
              refreshData(ref);
              await ref.read(summaryProvider.future);
            },
            child: summary.when(
              loading: () => const Loading(),
              error: (e, _) => ListView(children: [ErrorView(error: e, onRetry: () => ref.invalidate(summaryProvider))]),
              data: (s) => ListView(
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
                children: switch (section) {
                  BudgetSection.overview => _overview(context, ref, s),
                  BudgetSection.categories => _categories(context, s.budget),
                  BudgetSection.insights => _insights(s),
                },
              ),
            ),
          ),
        ),
      ]),
    );
  }

  void _openEdit(BuildContext context) =>
      Navigator.push(context, MaterialPageRoute(builder: (_) => const BudgetEditScreen()));

  // --- Overview -------------------------------------------------------------

  List<Widget> _overview(BuildContext context, WidgetRef ref, Summary s) {
    final b = s.budget;
    final spentCats = b.categories.where((c) => c.spent > 0).toList()..sort((a, c) => c.spent.compareTo(a.spent));
    final dismissed = ref.watch(_dismissedSuggestionProvider).contains(b.month);
    final suggestion = b.suggestion;
    return [
      AppCard(
        padding: const EdgeInsets.all(20),
        child: Row(children: [
          _Ring(pct: b.pctUsed),
          const SizedBox(width: 18),
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              const Text(S.spentThisMonth, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
              const SizedBox(height: 4),
              FittedBox(
                fit: BoxFit.scaleDown,
                child: Amount(rupiah(b.spentTotal), size: 24, weight: FontWeight.w800),
              ),
              if (b.totalLimit > 0) ...[
                Text('of ${rupiah(b.totalLimit)} budgeted',
                    style: const TextStyle(fontSize: 13, color: AppColors.muted, fontFeatures: tabular)),
                const SizedBox(height: 6),
                if (b.daysLeft > 0)
                  Tag(
                    b.remaining > 0 ? '${rupiah(b.perDayLeft)} / day left' : 'Over budget',
                    tone: b.remaining > 0 ? TagTone.green : TagTone.red,
                  ),
              ] else ...[
                const SizedBox(height: 6),
                TextButton(onPressed: () => _openEdit(context), child: const Text(S.setUpBudgetCta)),
              ],
            ]),
          ),
        ]),
      ),
      if (suggestion != null && suggestion.cuts.isNotEmpty && !dismissed) ...[
        const SizedBox(height: 16),
        _SuggestionCard(status: b, suggestion: suggestion),
      ],
      if (b.incomePlan == 0 && b.upcomingTotal > 0) ...[
        const SizedBox(height: 16),
        const Notice(text: S.incomeHint, icon: Icons.lightbulb_outline),
      ],
      if (b.upcoming.isNotEmpty) ...[
        const SizedBox(height: 16),
        AppCard(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
            const SectionHeader(S.upcoming),
            for (final u in b.upcoming)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 8),
                child: Row(children: [
                  const CategoryBadge(slug: 'debt', color: 'grey', size: 36),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                      Text(u.name, style: const TextStyle(fontWeight: FontWeight.w600)),
                      if (u.dueDate != null)
                        Text('Due ${shortDate(u.dueDate!)}', style: const TextStyle(fontSize: 13, color: AppColors.muted)),
                    ]),
                  ),
                  Amount(groupThousands(u.amount), size: 15),
                ]),
              ),
          ]),
        ),
      ],
      const SizedBox(height: 16),
      AppCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          const SectionHeader(S.whereItWent),
          const SizedBox(height: 12),
          if (spentCats.isEmpty) const Text(S.noTransactions, style: TextStyle(color: AppColors.muted)),
          if (spentCats.isNotEmpty) ...[
            ClipRRect(
              borderRadius: BorderRadius.circular(999),
              child: SizedBox(
                height: 14,
                child: Row(children: [
                  for (final c in spentCats)
                    Expanded(
                      flex: math.max(1, (c.spent * 1000 / math.max(1, b.spentTotal)).round()),
                      child: Container(
                        margin: const EdgeInsets.only(right: 3),
                        color: colorPair(c.color).$1,
                      ),
                    ),
                ]),
              ),
            ),
            const SizedBox(height: 8),
            for (final c in spentCats)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 9),
                child: Row(children: [
                  Container(
                    width: 12,
                    height: 12,
                    decoration: BoxDecoration(
                      color: colorPair(c.color).$1,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(color: const Color(0x1A0A0A0A)),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(child: Text(c.name, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15))),
                  SizedBox(
                    width: 44,
                    child: Text('${(c.spent * 100 / math.max(1, b.spentTotal)).round()}%',
                        textAlign: TextAlign.end,
                        style: const TextStyle(fontSize: 13, color: AppColors.muted, fontFeatures: tabular)),
                  ),
                  SizedBox(width: 96, child: Text(groupThousands(c.spent), textAlign: TextAlign.end, style: const TextStyle(fontWeight: FontWeight.w700, fontFeatures: tabular))),
                ]),
              ),
          ],
        ]),
      ),
      const SizedBox(height: 16),
      OutlinedButton.icon(
        onPressed: () => _openEdit(context),
        icon: const Icon(Icons.tune),
        label: const Text(S.editBudget),
      ),
    ];
  }

  // --- Categories -----------------------------------------------------------

  List<Widget> _categories(BuildContext context, MonthStatus b) {
    final rows = b.categories.where((c) => c.limit != null || c.spent > 0).toList();
    return [
      for (final c in rows) ...[
        _CategoryCard(c: c),
        const SizedBox(height: 12),
      ],
      OutlinedButton.icon(
        onPressed: () => _openEdit(context),
        icon: const Icon(Icons.add),
        label: const Text(S.setLimits),
      ),
    ];
  }

  // --- Insights -------------------------------------------------------------

  List<Widget> _insights(Summary s) {
    final days = s.daily;
    final now = DateTime.now();
    final isCurrent = s.month.year == now.year && s.month.month == now.month;
    final daysInMonth = DateTime(s.month.year, s.month.month + 1, 0).day;
    final elapsed = isCurrent ? now.day : daysInMonth;
    final total = days.fold<int>(0, (a, d) => a + d.amount);
    final avg = elapsed == 0 ? 0 : total ~/ elapsed;
    final biggest = days.isEmpty ? null : days.reduce((a, b) => a.amount >= b.amount ? a : b);
    final dailyBudget = s.budget.totalLimit == 0 ? null : s.budget.totalLimit ~/ daysInMonth;

    Widget stat(String label, String value, Widget tag) => Expanded(
          child: AppCard(
            padding: const EdgeInsets.all(14),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(label, style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
              const SizedBox(height: 4),
              FittedBox(fit: BoxFit.scaleDown, child: Amount(value, size: 20, weight: FontWeight.w800)),
              const SizedBox(height: 4),
              tag,
            ]),
          ),
        );

    return [
      Row(children: [
        stat(
          S.dailyAverage,
          rupiah(avg),
          dailyBudget == null
              ? const Tag('No budget set')
              : Tag(
                  avg <= dailyBudget ? 'Under ${rupiah(dailyBudget)}' : 'Over ${rupiah(dailyBudget)}',
                  tone: avg <= dailyBudget ? TagTone.green : TagTone.red,
                ),
        ),
        const SizedBox(width: 10),
        stat(
          S.biggestDay,
          biggest == null ? '–' : rupiah(biggest.amount),
          biggest == null ? const Tag('No spending yet') : Tag(shortDate(biggest.day), tone: TagTone.red),
        ),
      ]),
      const SizedBox(height: 16),
      AppCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          SectionHeader(S.dailySpending, action: '${monthName(s.month).substring(0, 3)} 1–$elapsed'),
          const SizedBox(height: 12),
          _DailyBars(days: days, upTo: elapsed, dailyBudget: dailyBudget),
        ]),
      ),
    ];
  }
}

class _Ring extends StatelessWidget {
  final int? pct;
  const _Ring({required this.pct});

  @override
  Widget build(BuildContext context) {
    final p = (pct ?? 0) / 100;
    return Semantics(
      label: pct == null ? 'No budget set' : '$pct percent of the monthly budget used',
      child: SizedBox(
        width: 120,
        height: 120,
        child: Stack(fit: StackFit.expand, children: [
          CircularProgressIndicator(
            value: p.clamp(0.0, 1.0),
            strokeWidth: 14,
            strokeCap: StrokeCap.round,
            backgroundColor: AppColors.track,
            color: p > 1 ? AppColors.red : AppColors.blue,
          ),
          Center(
            child: Column(mainAxisSize: MainAxisSize.min, children: [
              Text(pct == null ? '–' : '$pct%',
                  style: const TextStyle(fontSize: 26, fontWeight: FontWeight.w800, fontFeatures: tabular)),
              const Text(S.used, style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: AppColors.muted)),
            ]),
          ),
        ]),
      ),
    );
  }
}

class _SuggestionCard extends ConsumerStatefulWidget {
  final MonthStatus status;
  final Rebalance suggestion;
  const _SuggestionCard({required this.status, required this.suggestion});

  @override
  ConsumerState<_SuggestionCard> createState() => _SuggestionCardState();
}

class _SuggestionCardState extends ConsumerState<_SuggestionCard> {
  bool _busy = false;

  String _text() {
    final b = widget.status;
    final s = widget.suggestion;
    final cuts = s.cuts.map((c) => '${c.name} by ${rupiah(c.amount)}').toList();
    final cutText = cuts.length == 1 ? cuts.first : '${cuts.sublist(0, cuts.length - 1).join(', ')} and ${cuts.last}';
    final due = b.upcoming.isEmpty
        ? ''
        : b.upcoming.length == 1
            ? 'Your ${rupiah(b.upcoming.first.amount)} ${b.upcoming.first.name}'
                '${b.upcoming.first.dueDate == null ? '' : ' is due ${shortDate(b.upcoming.first.dueDate!)}'}. '
            : 'You have ${rupiah(b.upcomingTotal)} in installments left this month. ';
    final tail = s.uncovered > 0 ? ' That still leaves ${rupiah(s.uncovered)} short.' : ' to stay within your income.';
    return '${due}Cut $cutText$tail';
  }

  Future<void> _apply() async {
    setState(() => _busy = true);
    try {
      await ref.read(apiProvider).applySuggestion(apiMonth(widget.status.month));
      refreshData(ref);
    } on ApiException catch (e) {
      if (mounted) showSnack(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(color: AppColors.blueSoft, borderRadius: BorderRadius.circular(20)),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const Row(children: [
            CircleAvatar(
              radius: 17,
              backgroundColor: AppColors.blue,
              child: Icon(Icons.auto_awesome_outlined, size: 18, color: Colors.white),
            ),
            SizedBox(width: 10),
            Text(S.suggestion, style: TextStyle(fontWeight: FontWeight.w700, color: AppColors.blue)),
          ]),
          const SizedBox(height: 12),
          Text(_text(), style: const TextStyle(fontSize: 15, height: 1.45)),
          const SizedBox(height: 12),
          Row(children: [
            FilledButton(
              onPressed: _busy ? null : _apply,
              style: FilledButton.styleFrom(minimumSize: const Size(88, AppSizes.buttonSm)),
              child: const Text(S.apply),
            ),
            const SizedBox(width: 8),
            OutlinedButton(
              onPressed: () {
                final set = {...ref.read(_dismissedSuggestionProvider), widget.status.month};
                ref.read(_dismissedSuggestionProvider.notifier).state = set;
              },
              style: OutlinedButton.styleFrom(minimumSize: const Size(88, AppSizes.buttonSm)),
              child: const Text(S.notNow),
            ),
          ]),
        ]),
      );
}

class _CategoryCard extends StatelessWidget {
  final CategoryStatus c;
  const _CategoryCard({required this.c});

  @override
  Widget build(BuildContext context) {
    final limit = c.limit;
    final pct = c.pct ?? 0;
    final (tagText, tone) = limit == null
        ? (S.noLimit, TagTone.grey)
        : c.isOver
            ? ('Over', TagTone.red)
            : c.isNear
                ? ('Almost', TagTone.yellow)
                : ('$pct%', TagTone.grey);
    return AppCard(
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 14),
      child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Row(children: [
          CategoryBadge(slug: c.slug, color: c.color, size: 36),
          const SizedBox(width: 12),
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(c.name, style: const TextStyle(fontWeight: FontWeight.w700)),
              Text(
                limit == null ? rupiah(c.spent) : '${rupiah(c.spent)} of ${groupThousands(limit)}',
                style: const TextStyle(fontSize: 13, color: AppColors.muted, fontFeatures: tabular),
              ),
            ]),
          ),
          Tag(tagText, tone: tone),
        ]),
        if (limit != null) ...[
          const SizedBox(height: 10),
          ProgressBar(value: pct / 100, color: budgetColor(c.state), semanticsLabel: c.name),
          const SizedBox(height: 10),
          Text(
            c.isOver ? 'Over by ${rupiah(c.spent - limit)}' : '${rupiah(limit - c.spent)} left',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: c.isOver ? AppColors.redText : AppColors.muted,
              fontFeatures: tabular,
            ),
          ),
        ],
      ]),
    );
  }
}

/// Simple bar chart: one bar per day up to today, with the daily budget as a line.
class _DailyBars extends StatelessWidget {
  final List<DailyTotal> days;
  final int upTo;
  final int? dailyBudget;
  const _DailyBars({required this.days, required this.upTo, required this.dailyBudget});

  @override
  Widget build(BuildContext context) {
    final byDay = {for (final d in days) d.day.day: d.amount};
    final values = [for (var i = 1; i <= upTo; i++) byDay[i] ?? 0];
    final maxV = math.max(values.fold<int>(0, (a, b) => math.max(a, b)), dailyBudget ?? 0);
    if (maxV == 0) {
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: 24),
        child: Text(S.noTransactions, style: TextStyle(color: AppColors.muted)),
      );
    }
    const h = 180.0;
    return Semantics(
      label: 'Daily spending chart, $upTo days',
      child: SizedBox(
        height: h + 20,
        child: LayoutBuilder(builder: (context, box) {
          final barW = math.max(4.0, math.min(14.0, box.maxWidth / values.length - 4));
          return Stack(children: [
            Positioned.fill(
              bottom: 20,
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.end,
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  for (final v in values)
                    Tooltip(
                      message: rupiah(v),
                      child: Container(
                        width: barW,
                        height: math.max(2.0, h * v / maxV),
                        decoration: BoxDecoration(
                          color: dailyBudget != null && v > dailyBudget! ? AppColors.red : AppColors.blue,
                          borderRadius: BorderRadius.circular(4),
                        ),
                      ),
                    ),
                ],
              ),
            ),
            if (dailyBudget != null)
              Positioned(
                left: 0,
                right: 0,
                bottom: 20 + h * dailyBudget! / maxV,
                child: Container(height: 1.5, color: AppColors.muted),
              ),
            Positioned(
              left: 0,
              right: 0,
              bottom: 0,
              child: Row(mainAxisAlignment: MainAxisAlignment.spaceBetween, children: [
                const Text('1', style: TextStyle(fontSize: 11, color: AppColors.muted)),
                if (dailyBudget != null)
                  Text('— daily budget ${rupiah(dailyBudget!)}', style: const TextStyle(fontSize: 11, color: AppColors.muted)),
                Text('$upTo', style: const TextStyle(fontSize: 11, color: AppColors.muted)),
              ]),
            ),
          ]);
        }),
      ),
    );
  }
}
