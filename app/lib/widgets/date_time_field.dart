import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../theme/tokens.dart';
import '../util/format.dart';

/// Tappable field showing "Thu, Oct 15 · 19:05" that opens date then time pickers.
class DateTimeField extends StatelessWidget {
  final DateTime value;
  final ValueChanged<DateTime> onChanged;
  final bool highlight;
  final String? helper;
  const DateTimeField({super.key, required this.value, required this.onChanged, this.highlight = false, this.helper});

  Future<void> _pick(BuildContext context) async {
    final date = await showDatePicker(
      context: context,
      initialDate: value,
      firstDate: DateTime(2020),
      lastDate: DateTime.now().add(const Duration(days: 1)),
    );
    if (date == null || !context.mounted) return;
    final time = await showTimePicker(context: context, initialTime: TimeOfDay.fromDateTime(value));
    final t = time ?? TimeOfDay.fromDateTime(value);
    onChanged(DateTime(date.year, date.month, date.day, t.hour, t.minute));
  }

  @override
  Widget build(BuildContext context) => InkWell(
        borderRadius: BorderRadius.circular(AppRadius.field),
        onTap: () => _pick(context),
        child: InputDecorator(
          decoration: InputDecoration(
            labelText: S.date,
            helperText: helper,
            helperStyle: const TextStyle(color: AppColors.yellowText),
            fillColor: highlight ? AppColors.yellowSoft : AppColors.card,
            suffixIcon: const Icon(Icons.calendar_today_outlined, size: 20),
          ),
          child: Text(dateTime(value), style: const TextStyle(fontWeight: FontWeight.w500, fontSize: 16)),
        ),
      );
}
