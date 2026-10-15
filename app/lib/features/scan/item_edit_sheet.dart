import 'package:flutter/material.dart';

import '../../api/models.dart';
import '../../l10n/strings.dart';
import '../../util/format.dart';

/// Bottom sheet to fix one receipt line. Returns the edited copy, or null.
Future<LineItem?> showItemEditSheet(BuildContext context, LineItem item) => showModalBottomSheet<LineItem>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => _ItemEditSheet(item: item),
    );

class _ItemEditSheet extends StatefulWidget {
  final LineItem item;
  const _ItemEditSheet({required this.item});

  @override
  State<_ItemEditSheet> createState() => _ItemEditSheetState();
}

class _ItemEditSheetState extends State<_ItemEditSheet> {
  late final TextEditingController _name;
  late final TextEditingController _qty;
  late final TextEditingController _price;
  late final TextEditingController _total;

  @override
  void initState() {
    super.initState();
    final i = widget.item;
    _name = TextEditingController(text: i.name);
    _qty = TextEditingController(text: i.qty == i.qty.roundToDouble() ? i.qty.toInt().toString() : '${i.qty}');
    _price = TextEditingController(text: i.unitPrice == null ? '' : groupThousands(i.unitPrice!));
    _total = TextEditingController(text: groupThousands(i.total));
  }

  @override
  void dispose() {
    for (final c in [_name, _qty, _price, _total]) {
      c.dispose();
    }
    super.dispose();
  }

  double get _q => double.tryParse(_qty.text.replaceAll(',', '.')) ?? 1;

  /// Keep the line total consistent when quantity or price changes.
  void _recalc() {
    final p = parseRupiah(_price.text);
    if (p != null) _total.text = groupThousands((p * _q).round());
  }

  @override
  Widget build(BuildContext context) => Padding(
        padding: EdgeInsets.fromLTRB(16, 0, 16, 16 + MediaQuery.of(context).viewInsets.bottom),
        child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.stretch, children: [
          const Text('Edit item', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          const SizedBox(height: 16),
          TextField(controller: _name, decoration: const InputDecoration(labelText: 'Item')),
          const SizedBox(height: 12),
          Row(children: [
            SizedBox(
              width: 90,
              child: TextField(
                controller: _qty,
                keyboardType: const TextInputType.numberWithOptions(decimal: true),
                decoration: const InputDecoration(labelText: 'Qty'),
                onChanged: (_) => _recalc(),
              ),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: TextField(
                controller: _price,
                keyboardType: TextInputType.number,
                inputFormatters: [RupiahInputFormatter()],
                decoration: const InputDecoration(labelText: 'Price each', prefixText: 'Rp '),
                onChanged: (_) => _recalc(),
              ),
            ),
          ]),
          const SizedBox(height: 12),
          TextField(
            controller: _total,
            keyboardType: TextInputType.number,
            inputFormatters: [RupiahInputFormatter()],
            decoration: const InputDecoration(labelText: 'Line total', prefixText: 'Rp '),
          ),
          const SizedBox(height: 16),
          FilledButton(
            onPressed: () => Navigator.pop(
              context,
              LineItem(
                name: _name.text.trim(),
                qty: _q,
                unitPrice: parseRupiah(_price.text),
                total: parseRupiah(_total.text) ?? 0,
                confidence: 1,
                // Fixed by a human: no longer flagged.
                flagged: false,
                included: widget.item.included,
              ),
            ),
            child: const Text(S.save),
          ),
        ]),
      );
}
