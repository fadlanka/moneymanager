import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../state/providers.dart';
import '../../theme/app_theme.dart';
import '../../theme/tokens.dart';
import '../../util/format.dart';
import '../../widgets/category_picker.dart';
import '../../widgets/common.dart';
import '../../widgets/date_time_field.dart';
import 'item_edit_sheet.dart';

/// Step 2: the user checks what the AI read. Anything the backend flagged
/// (low confidence or failed arithmetic) is highlighted in yellow.
class ReviewScreen extends ConsumerStatefulWidget {
  final Receipt receipt;
  final Uint8List imageBytes;
  const ReviewScreen({super.key, required this.receipt, required this.imageBytes});

  @override
  ConsumerState<ReviewScreen> createState() => _ReviewScreenState();
}

class _ReviewScreenState extends ConsumerState<ReviewScreen> {
  late final TextEditingController _merchant;
  late final TextEditingController _total;
  late DateTime _date;
  late bool _dateFound;
  String? _categoryId;
  late List<LineItem> _items;
  bool _busy = false;

  Extraction? get _ex => widget.receipt.extraction;
  List<Issue> get _issues => widget.receipt.issues;
  bool _issueOn(String field) => _issues.any((i) => i.field == field);
  String? _issueText(String field) => _issues.where((i) => i.field == field).map((i) => i.message).firstOrNull;

  @override
  void initState() {
    super.initState();
    final ex = _ex;
    _merchant = TextEditingController(text: ex?.merchant ?? '');
    _total = TextEditingController(text: ex?.total == null ? '' : groupThousands(ex!.total!));
    _dateFound = ex?.date != null;
    _date = ex?.date?.toLocal() ?? DateTime.now();
    _categoryId = widget.receipt.suggestedCategoryId;
    // Copies, so edits don't touch the receipt object.
    _items = [
      for (final i in ex?.items ?? const <LineItem>[])
        LineItem(
          name: i.name,
          qty: i.qty,
          unitPrice: i.unitPrice,
          total: i.total,
          confidence: i.confidence,
          flagged: i.flagged,
        ),
    ];
  }

  @override
  void dispose() {
    _merchant.dispose();
    _total.dispose();
    super.dispose();
  }

  int get _itemsSum => _items.where((i) => i.included).fold(0, (s, i) => s + i.total);

  /// Items, minus discount, plus tax and service: what the total should be.
  int get _expectedTotal {
    final ex = _ex;
    return _itemsSum - (ex?.discount ?? 0).abs() + (ex?.tax ?? 0) + (ex?.service ?? 0);
  }

  Future<void> _editItem(int index) async {
    final updated = await showItemEditSheet(context, _items[index]);
    if (updated != null) setState(() => _items[index] = updated);
  }

  Future<void> _save({required bool checkLater}) async {
    final amount = parseRupiah(_total.text) ?? 0;
    if (amount <= 0) {
      showSnack(context, 'Enter the total.');
      return;
    }
    if (_merchant.text.trim().isEmpty) {
      showSnack(context, 'Enter the store name.');
      return;
    }
    setState(() => _busy = true);
    try {
      await ref.read(apiProvider).createTransaction({
        'kind': 'expense',
        'amount': amount,
        'merchant': _merchant.text.trim(),
        'occurred_at': _date.toUtc().toIso8601String(),
        'category_id': _categoryId,
        'receipt_id': widget.receipt.id,
        'items': _items.where((i) => i.included).map((i) => i.toJson()).toList(),
        'needs_check': checkLater,
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

  void _viewPhoto() => Navigator.push(
        context,
        MaterialPageRoute(
          builder: (_) => Scaffold(
            backgroundColor: Colors.black,
            appBar: AppBar(backgroundColor: Colors.black, foregroundColor: Colors.white),
            body: InteractiveViewer(
              maxScale: 6,
              child: Center(child: Image.memory(widget.imageBytes, errorBuilder: (context, error, stack) => const SizedBox())),
            ),
          ),
        ),
      );

  @override
  Widget build(BuildContext context) {
    final ex = _ex;
    final cats = (ref.watch(categoriesProvider).valueOrNull ?? const <Category>[])
        .where((c) => c.kind == 'expense')
        .toList();
    final flaggedItems = _items.where((i) => i.flagged).length;
    final fieldIssues = _issues.where((i) => !i.field.startsWith('items')).length;
    final toCheck = flaggedItems + fieldIssues;
    final total = parseRupiah(_total.text) ?? 0;
    final mismatch = _items.isNotEmpty && (total - _expectedTotal).abs() > 100;

    return Scaffold(
      appBar: AppBar(
        title: const Column(mainAxisSize: MainAxisSize.min, children: [
          Text(S.reviewReceipt, style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700)),
          Text(S.step2, style: TextStyle(fontSize: 12, color: AppColors.muted)),
        ]),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        children: [
          if (ex != null && !ex.isReceipt)
            const Notice(text: S.notReceipt, bg: AppColors.redSoft, iconBg: AppColors.red)
          else
            Notice(
              text: toCheck == 0
                  ? 'AI read ${_items.length} items and everything adds up.'
                  : 'AI read ${_items.length} items. $toCheck ${toCheck == 1 ? 'thing needs' : 'things need'} a check.',
              icon: toCheck == 0 ? Icons.check_rounded : Icons.priority_high_rounded,
              bg: toCheck == 0 ? AppColors.greenSoft : AppColors.yellowSoft,
              iconBg: toCheck == 0 ? AppColors.green : AppColors.yellow,
            ),
          const SizedBox(height: 16),
          AppCard(
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Semantics(
                  button: true,
                  label: 'View receipt photo',
                  child: GestureDetector(
                    onTap: _viewPhoto,
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(12),
                      child: Image.memory(
                        widget.imageBytes,
                        width: 64,
                        height: 86,
                        fit: BoxFit.cover,
                        errorBuilder: (context, error, stack) =>
                            Container(width: 64, height: 86, color: AppColors.track, child: const Icon(Icons.receipt_long)),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _FlagField(
                    flagged: _issueOn('merchant'),
                    helper: _issueText('merchant'),
                    child: TextField(
                      controller: _merchant,
                      textCapitalization: TextCapitalization.words,
                      decoration: InputDecoration(
                        labelText: S.store,
                        fillColor: _issueOn('merchant') ? AppColors.yellowSoft : AppColors.card,
                      ),
                    ),
                  ),
                ),
              ]),
              const SizedBox(height: 12),
              DateTimeField(
                value: _date,
                onChanged: (d) => setState(() {
                  _date = d;
                  _dateFound = true;
                }),
                highlight: _issueOn('date'),
                helper: _issueText('date') ?? (_dateFound ? null : 'Not on the receipt; using now.'),
              ),
              const SizedBox(height: 16),
              const Text(S.category,
                  style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: AppColors.muted)),
              const SizedBox(height: 8),
              CategoryPicker(
                categories: cats,
                selectedId: _categoryId,
                onSelect: (id) => setState(() => _categoryId = id),
              ),
            ]),
          ),
          const SizedBox(height: 16),
          AppCard(
            padding: const EdgeInsets.fromLTRB(8, 8, 16, 12),
            child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
              if (_items.isEmpty)
                const Padding(
                  padding: EdgeInsets.all(8),
                  child: Text('No items were read. You can still save the total.',
                      style: TextStyle(color: AppColors.muted)),
                ),
              for (var i = 0; i < _items.length; i++)
                _ItemRow(
                  item: _items[i],
                  onToggle: (v) => setState(() => _items[i].included = v),
                  onTap: () => _editItem(i),
                ),
              if (ex != null && ((ex.discount ?? 0) != 0 || (ex.tax ?? 0) != 0 || (ex.service ?? 0) != 0)) ...[
                const Divider(),
                if ((ex.discount ?? 0) != 0) _SubLine('Discount', '−${groupThousands(ex.discount!.abs())}'),
                if ((ex.tax ?? 0) != 0) _SubLine('Tax', groupThousands(ex.tax!)),
                if ((ex.service ?? 0) != 0) _SubLine('Service', groupThousands(ex.service!)),
              ],
              const Divider(),
              const SizedBox(height: 8),
              Padding(
                padding: const EdgeInsets.only(left: 8),
                child: _FlagField(
                  flagged: _issueOn('total'),
                  helper: _issueText('total'),
                  child: TextField(
                    controller: _total,
                    keyboardType: TextInputType.number,
                    inputFormatters: [RupiahInputFormatter()],
                    onChanged: (_) => setState(() {}),
                    style: const TextStyle(fontSize: 22, fontWeight: FontWeight.w800, fontFeatures: tabular),
                    decoration: InputDecoration(
                      labelText: S.total,
                      prefixText: 'Rp ',
                      fillColor: _issueOn('total') ? AppColors.yellowSoft : AppColors.card,
                    ),
                  ),
                ),
              ),
              if (mismatch) ...[
                const SizedBox(height: 10),
                Padding(
                  padding: const EdgeInsets.only(left: 8),
                  child: Row(children: [
                    Expanded(
                      child: Text('${S.itemsAddUp} ${rupiah(_expectedTotal)}.',
                          style: const TextStyle(fontSize: 13, color: AppColors.yellowText, fontWeight: FontWeight.w600)),
                    ),
                    TextButton(
                      onPressed: () => setState(() => _total.text = groupThousands(_expectedTotal)),
                      child: const Text(S.useItemsTotal),
                    ),
                  ]),
                ),
              ],
            ]),
          ),
          if (widget.receipt.model != null) ...[
            const SizedBox(height: 12),
            Text(
              'Read by ${widget.receipt.model}'
              '${widget.receipt.latencyMs == null ? '' : ' in ${(widget.receipt.latencyMs! / 1000).toStringAsFixed(1)}s'}',
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 12, color: AppColors.muted),
            ),
          ],
        ],
      ),
      bottomNavigationBar: SafeArea(
        child: Container(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
          decoration: const BoxDecoration(
            color: AppColors.card,
            border: Border(top: BorderSide(color: AppColors.hair)),
          ),
          child: Row(children: [
            Expanded(
              child: OutlinedButton(
                onPressed: _busy ? null : () => _save(checkLater: true),
                child: const Text(S.saveCheckLater, textAlign: TextAlign.center),
              ),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: FilledButton(
                onPressed: _busy ? null : () => _save(checkLater: false),
                child: _busy
                    ? const SizedBox(
                        width: 22, height: 22, child: CircularProgressIndicator(strokeWidth: 2.5, color: Colors.white))
                    : const Text(S.save),
              ),
            ),
          ]),
        ),
      ),
    );
  }
}

class _FlagField extends StatelessWidget {
  final bool flagged;
  final String? helper;
  final Widget child;
  const _FlagField({required this.flagged, required this.helper, required this.child});

  @override
  Widget build(BuildContext context) => Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        child,
        if (flagged && helper != null)
          Padding(
            padding: const EdgeInsets.only(top: 6, left: 4),
            child: Text(helper!,
                style: const TextStyle(fontSize: 12, color: AppColors.yellowText, fontWeight: FontWeight.w600)),
          ),
      ]);
}

class _ItemRow extends StatelessWidget {
  final LineItem item;
  final ValueChanged<bool> onToggle;
  final VoidCallback onTap;
  const _ItemRow({required this.item, required this.onToggle, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final qty = item.qty == item.qty.roundToDouble() ? item.qty.toInt().toString() : item.qty.toString();
    final sub = item.flagged
        ? S.blurry
        : item.unitPrice == null
            ? 'Qty $qty'
            : '$qty × Rp ${groupThousands(item.unitPrice!)}';
    return Material(
      color: item.flagged ? AppColors.yellowSoft : Colors.transparent,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 4),
          child: Row(children: [
            Checkbox(
              value: item.included,
              onChanged: (v) => onToggle(v ?? true),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
              activeColor: AppColors.blue,
              semanticLabel: 'Include ${item.name}',
            ),
            Expanded(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text(item.name.isEmpty ? '(no name)' : item.name,
                    style: TextStyle(
                      fontWeight: FontWeight.w700,
                      color: item.included ? AppColors.ink : AppColors.muted,
                      decoration: item.included ? null : TextDecoration.lineThrough,
                    )),
                Text(sub,
                    style: TextStyle(
                      fontSize: 13,
                      color: item.flagged ? AppColors.yellowText : AppColors.muted,
                    )),
              ]),
            ),
            Amount(groupThousands(item.total), color: item.included ? AppColors.ink : AppColors.muted),
          ]),
        ),
      ),
    );
  }
}

class _SubLine extends StatelessWidget {
  final String label;
  final String value;
  const _SubLine(this.label, this.value);

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 0, 0),
        child: Row(children: [
          Expanded(child: Text(label, style: const TextStyle(color: AppColors.muted))),
          Amount(value, size: 15, weight: FontWeight.w600, color: AppColors.muted),
        ]),
      );
}
