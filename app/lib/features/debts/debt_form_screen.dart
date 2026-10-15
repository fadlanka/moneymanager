import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/common.dart';

class DebtFormScreen extends ConsumerStatefulWidget {
  final Debt? existing;
  const DebtFormScreen({super.key, this.existing});

  @override
  ConsumerState<DebtFormScreen> createState() => _DebtFormScreenState();
}

class _DebtFormScreenState extends ConsumerState<DebtFormScreen> {
  final _form = GlobalKey<FormState>();
  late final TextEditingController _name;
  late final TextEditingController _creditor;
  late final TextEditingController _principal;
  late final TextEditingController _balance;
  late final TextEditingController _monthly;
  late final TextEditingController _rate;
  late final TextEditingController _dueDay;
  late String _kind;
  bool _busy = false;

  bool get _editing => widget.existing != null;

  @override
  void initState() {
    super.initState();
    final d = widget.existing;
    String money(int? v) => v == null || v == 0 ? '' : groupThousands(v);
    _name = TextEditingController(text: d?.name ?? '');
    _creditor = TextEditingController(text: d?.creditor ?? '');
    _principal = TextEditingController(text: money(d?.principal));
    _balance = TextEditingController(text: d == null ? '' : groupThousands(d.balance));
    _monthly = TextEditingController(text: money(d?.monthlyPayment));
    _rate = TextEditingController(
      text: d == null || d.interestRateBp == 0 ? '' : (d.interestRateBp / 100).toString().replaceAll('.', ','),
    );
    _dueDay = TextEditingController(text: d?.dueDay?.toString() ?? '');
    _kind = d?.kind ?? 'installment';
  }

  @override
  void dispose() {
    for (final c in [_name, _creditor, _principal, _balance, _monthly, _rate, _dueDay]) {
      c.dispose();
    }
    super.dispose();
  }

  /// "12,5" or "12.5" percent per year -> 1250 basis points.
  int _rateBp() {
    final v = double.tryParse(_rate.text.trim().replaceAll(',', '.'));
    return v == null ? 0 : (v * 100).round();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    final principal = parseRupiah(_principal.text)!;
    final body = {
      'name': _name.text.trim(),
      'kind': _kind,
      'creditor': _creditor.text.trim().isEmpty ? null : _creditor.text.trim(),
      'principal': principal,
      'balance': parseRupiah(_balance.text) ?? principal,
      'monthly_payment': parseRupiah(_monthly.text) ?? 0,
      'interest_rate_bp': _rateBp(),
      'due_day': int.tryParse(_dueDay.text.trim()),
    };
    final api = ref.read(apiProvider);
    try {
      if (_editing) {
        await api.updateDebt(widget.existing!.id, body);
      } else {
        await api.createDebt(body);
      }
      refreshData(ref);
      if (mounted) Navigator.pop(context);
    } on ApiException catch (e) {
      if (mounted) showSnack(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _delete() async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Delete ${widget.existing!.name}?'),
        content: const Text('Payments already recorded as transactions stay in Activity.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text(S.cancel)),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text(S.delete, style: TextStyle(color: AppColors.redText)),
          ),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await ref.read(apiProvider).deleteDebt(widget.existing!.id);
      refreshData(ref);
      if (mounted) Navigator.pop(context);
    } on ApiException catch (e) {
      if (mounted) showSnack(context, e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    String? requiredField(String? v) => (v ?? '').trim().isEmpty ? S.required : null;
    InputDecoration money(String label) => InputDecoration(labelText: label, prefixText: 'Rp ');

    return Scaffold(
      appBar: AppBar(
        title: Text(_editing ? S.editDebt : S.newDebt, style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
        actions: [
          if (_editing) IconButton(tooltip: S.delete, onPressed: _delete, icon: const Icon(Icons.delete_outline)),
        ],
      ),
      body: Form(
        key: _form,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
          children: [
            Segmented<String>(
              options: const [('installment', S.installment), ('loan', S.loan)],
              value: _kind,
              onChanged: (v) => setState(() => _kind = v),
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _name,
              textCapitalization: TextCapitalization.sentences,
              decoration: const InputDecoration(labelText: S.debtName),
              validator: requiredField,
            ),
            const SizedBox(height: 12),
            TextFormField(controller: _creditor, decoration: const InputDecoration(labelText: S.creditor)),
            const SizedBox(height: 12),
            TextFormField(
              controller: _principal,
              keyboardType: TextInputType.number,
              inputFormatters: [RupiahInputFormatter()],
              decoration: money(S.principal),
              validator: (v) => (parseRupiah(v ?? '') ?? 0) <= 0 ? S.required : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _balance,
              keyboardType: TextInputType.number,
              inputFormatters: [RupiahInputFormatter()],
              decoration: money(S.balance).copyWith(helperText: 'Leave empty if nothing is paid yet'),
              validator: (v) {
                final b = parseRupiah(v ?? '');
                final p = parseRupiah(_principal.text) ?? 0;
                return b != null && b > p ? 'Can\'t be more than the original amount' : null;
              },
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _monthly,
              keyboardType: TextInputType.number,
              inputFormatters: [RupiahInputFormatter()],
              decoration: money(S.monthlyPayment),
            ),
            const SizedBox(height: 12),
            Row(children: [
              Expanded(
                child: TextFormField(
                  controller: _rate,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(labelText: S.interestRate, suffixText: '%'),
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: TextFormField(
                  controller: _dueDay,
                  keyboardType: TextInputType.number,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly, LengthLimitingTextInputFormatter(2)],
                  decoration: const InputDecoration(labelText: S.dueDay),
                  validator: (v) {
                    if ((v ?? '').isEmpty) return null;
                    final d = int.tryParse(v!);
                    return d == null || d < 1 || d > 31 ? '1–31' : null;
                  },
                ),
              ),
            ]),
          ],
        ),
      ),
      bottomNavigationBar: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
          child: FilledButton(onPressed: _busy ? null : _save, child: const Text(S.save)),
        ),
      ),
    );
  }
}
