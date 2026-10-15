import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';

/// Monthly income plus a limit per expense category for the selected month.
class BudgetEditScreen extends ConsumerStatefulWidget {
  const BudgetEditScreen({super.key});

  @override
  ConsumerState<BudgetEditScreen> createState() => _BudgetEditScreenState();
}

class _BudgetEditScreenState extends ConsumerState<BudgetEditScreen> {
  final _income = TextEditingController();
  final Map<String, TextEditingController> _limits = {};
  bool _loaded = false;
  bool _busy = false;

  @override
  void dispose() {
    _income.dispose();
    for (final c in _limits.values) {
      c.dispose();
    }
    super.dispose();
  }

  void _fill(BudgetList list, int income) {
    if (_loaded) return;
    _loaded = true;
    _income.text = income == 0 ? '' : groupThousands(income);
    for (final r in list.items) {
      _limits[r.categoryId] = TextEditingController(text: r.limit == null ? '' : groupThousands(r.limit!));
    }
  }

  Future<void> _save(BudgetList list) async {
    setState(() => _busy = true);
    final api = ref.read(apiProvider);
    try {
      final user = await api.updateMe(monthlyIncome: parseRupiah(_income.text) ?? 0);
      ref.read(authProvider.notifier).updateUser(user);
      await api.saveBudgets(apiMonth(list.month), {
        for (final r in list.items) r.categoryId: parseRupiah(_limits[r.categoryId]!.text),
      });
      refreshData(ref);
      if (mounted) {
        showSnack(context, S.saved);
        Navigator.pop(context);
      }
    } on ApiException catch (e) {
      if (mounted) showSnack(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final budgets = ref.watch(budgetsProvider);
    final income = ref.watch(authProvider).user?.monthlyIncome ?? 0;
    return Scaffold(
      appBar: AppBar(title: const Text(S.editBudget, style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700))),
      body: budgets.when(
        loading: () => const Loading(),
        error: (e, _) => ErrorView(error: e, onRetry: () => ref.invalidate(budgetsProvider)),
        data: (list) {
          _fill(list, income);
          final total = list.items.fold<int>(0, (s, r) => s + (parseRupiah(_limits[r.categoryId]!.text) ?? 0));
          final incomeValue = parseRupiah(_income.text) ?? 0;
          return ListView(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
            children: [
              Text(monthYear(list.month),
                  style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
              const SizedBox(height: 8),
              if (list.copiedFrom != null) ...[
                Notice(
                  text: 'Pre-filled from ${monthYear(list.copiedFrom!)}. Save to use these limits this month.',
                  icon: Icons.content_copy_outlined,
                  bg: AppColors.blueSoft,
                  iconBg: AppColors.card,
                ),
                const SizedBox(height: 12),
              ],
              TextField(
                controller: _income,
                keyboardType: TextInputType.number,
                inputFormatters: [RupiahInputFormatter()],
                onChanged: (_) => setState(() {}),
                decoration: const InputDecoration(labelText: S.monthlyIncome, prefixText: 'Rp '),
              ),
              const SizedBox(height: 8),
              Text(
                incomeValue == 0
                    ? 'Budgeted: ${rupiah(total)}'
                    : 'Budgeted ${rupiah(total)} of ${rupiah(incomeValue)}'
                        '${total > incomeValue ? ' — more than your income' : ''}',
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                  color: incomeValue > 0 && total > incomeValue ? AppColors.redText : AppColors.muted,
                ),
              ),
              const SizedBox(height: 16),
              AppCard(
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
                child: Column(children: [
                  for (var i = 0; i < list.items.length; i++) ...[
                    if (i > 0) const Divider(),
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                      child: Row(children: [
                        CategoryBadge(slug: list.items[i].slug, color: list.items[i].color, size: 36),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                            Text(list.items[i].name, style: const TextStyle(fontWeight: FontWeight.w600)),
                            Text('Spent ${rupiah(list.items[i].spent)}',
                                style: const TextStyle(fontSize: 12, color: AppColors.muted)),
                          ]),
                        ),
                        SizedBox(
                          width: 140,
                          child: TextField(
                            controller: _limits[list.items[i].categoryId],
                            keyboardType: TextInputType.number,
                            textAlign: TextAlign.end,
                            inputFormatters: [RupiahInputFormatter()],
                            onChanged: (_) => setState(() {}),
                            decoration: const InputDecoration(hintText: S.noLimit, isDense: true, prefixText: 'Rp '),
                          ),
                        ),
                      ]),
                    ),
                  ],
                ]),
              ),
            ],
          );
        },
      ),
      bottomNavigationBar: budgets.valueOrNull == null
          ? null
          : SafeArea(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
                child: FilledButton(
                  onPressed: _busy ? null : () => _save(budgets.value!),
                  child: const Text(S.save),
                ),
              ),
            ),
    );
  }
}
