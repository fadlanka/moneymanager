import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/category_picker.dart';
import '../../widgets/common.dart';
import '../../widgets/date_time_field.dart';

/// Add a transaction by hand, or edit an existing one.
class TransactionFormScreen extends ConsumerStatefulWidget {
  final Transaction? existing;
  const TransactionFormScreen({super.key, this.existing});

  @override
  ConsumerState<TransactionFormScreen> createState() => _TransactionFormScreenState();
}

class _TransactionFormScreenState extends ConsumerState<TransactionFormScreen> {
  final _form = GlobalKey<FormState>();
  late final TextEditingController _amount;
  late final TextEditingController _merchant;
  late final TextEditingController _note;
  late String _kind;
  late DateTime _when;
  String? _categoryId;
  bool _busy = false;

  bool get _editing => widget.existing != null;

  @override
  void initState() {
    super.initState();
    final t = widget.existing;
    _amount = TextEditingController(text: t == null ? '' : groupThousands(t.amount));
    _merchant = TextEditingController(text: t?.merchant ?? '');
    _note = TextEditingController(text: t?.note ?? '');
    _kind = t?.kind ?? 'expense';
    _when = t?.occurredAt.toLocal() ?? DateTime.now();
    _categoryId = t?.categoryId;
  }

  @override
  void dispose() {
    _amount.dispose();
    _merchant.dispose();
    _note.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _busy = true);
    final api = ref.read(apiProvider);
    final amount = parseRupiah(_amount.text)!;
    try {
      if (_editing) {
        await api.updateTransaction(widget.existing!.id, {
          'amount': amount,
          'merchant': _merchant.text.trim(),
          'note': _note.text.trim().isEmpty ? null : _note.text.trim(),
          'category_id': _categoryId,
          'occurred_at': _when.toUtc().toIso8601String(),
        });
      } else {
        await api.createTransaction({
          'kind': _kind,
          'amount': amount,
          'merchant': _merchant.text.trim(),
          'note': _note.text.trim().isEmpty ? null : _note.text.trim(),
          'category_id': _categoryId,
          'occurred_at': _when.toUtc().toIso8601String(),
        });
      }
      refreshData(ref);
      if (mounted) {
        showSnack(context, S.saved);
        Navigator.pop(context, true);
      }
    } on ApiException catch (e) {
      if (mounted) showSnack(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final cats = (ref.watch(categoriesProvider).valueOrNull ?? const <Category>[])
        .where((c) => c.kind == _kind)
        .toList();
    return Scaffold(
      appBar: AppBar(
        title: Text(_editing ? S.editTransaction : S.newTransaction,
            style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
      ),
      body: Form(
        key: _form,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
          children: [
            if (!_editing) ...[
              Segmented<String>(
                options: const [('expense', S.expense), ('income', S.income)],
                value: _kind,
                onChanged: (v) => setState(() {
                  _kind = v;
                  _categoryId = null;
                }),
              ),
              const SizedBox(height: 16),
            ],
            TextFormField(
              controller: _amount,
              autofocus: !_editing,
              keyboardType: TextInputType.number,
              inputFormatters: [RupiahInputFormatter()],
              style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w800),
              decoration: const InputDecoration(labelText: S.amount, prefixText: 'Rp '),
              validator: (v) => (parseRupiah(v ?? '') ?? 0) <= 0 ? S.required : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _merchant,
              textCapitalization: TextCapitalization.sentences,
              decoration: const InputDecoration(labelText: S.merchant),
              validator: (v) => (v ?? '').trim().isEmpty ? S.required : null,
            ),
            const SizedBox(height: 16),
            const Text(S.category, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
            const SizedBox(height: 8),
            CategoryPicker(
              categories: cats,
              selectedId: _categoryId,
              onSelect: (id) => setState(() => _categoryId = id),
            ),
            const SizedBox(height: 16),
            DateTimeField(value: _when, onChanged: (d) => setState(() => _when = d)),
            const SizedBox(height: 12),
            TextFormField(
              controller: _note,
              maxLines: 2,
              decoration: const InputDecoration(labelText: S.note),
            ),
          ],
        ),
      ),
      bottomNavigationBar: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 12),
          child: FilledButton(
            onPressed: _busy ? null : _save,
            child: _busy
                ? const SizedBox(width: 22, height: 22, child: CircularProgressIndicator(strokeWidth: 2.5, color: Colors.white))
                : const Text(S.save),
          ),
        ),
      ),
    );
  }
}
