import 'package:flutter/services.dart';
import 'package:intl/intl.dart';

/// 4250000 -> "4.250.000" (Indonesian thousands separator).
String groupThousands(int value) {
  final digits = value.abs().toString();
  final buf = StringBuffer();
  for (var i = 0; i < digits.length; i++) {
    if (i > 0 && (digits.length - i) % 3 == 0) buf.write('.');
    buf.write(digits[i]);
  }
  return '${value < 0 ? '-' : ''}$buf';
}

/// 4250000 -> "Rp 4.250.000"
String rupiah(int value) => '${value < 0 ? '−' : ''}Rp ${groupThousands(value.abs())}';

/// Signed amount for lists: expenses "−87.400", income "+1.500.000".
String signedAmount(int amount, {required bool income}) =>
    '${income ? '+' : '−'}${groupThousands(amount)}';

/// Digits only; "Rp 25.000" -> 25000. Returns null when empty.
int? parseRupiah(String text) {
  final digits = text.replaceAll(RegExp(r'[^0-9]'), '');
  if (digits.isEmpty) return null;
  return int.tryParse(digits);
}

final _monthYear = DateFormat('MMM yyyy');
final _dayHeader = DateFormat('EEE, MMM d');
final _dateTime = DateFormat('EEE, MMM d · HH:mm');
final _time = DateFormat('HH:mm');
final _shortDate = DateFormat('MMM d');
final _monthName = DateFormat('MMMM');

String monthYear(DateTime d) => _monthYear.format(d);
String monthName(DateTime d) => _monthName.format(d);
String dateTime(DateTime d) => _dateTime.format(d.toLocal());
String timeOnly(DateTime d) => _time.format(d.toLocal());
String shortDate(DateTime d) => _shortDate.format(d.toLocal());

/// "Today", "Yesterday", or "Wed, Oct 14".
String dayLabel(DateTime d) {
  final local = d.toLocal();
  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);
  final day = DateTime(local.year, local.month, local.day);
  final diff = today.difference(day).inDays;
  if (diff == 0) return 'Today';
  if (diff == 1) return 'Yesterday';
  return _dayHeader.format(local);
}

/// "2026-10" for API month parameters.
String apiMonth(DateTime d) => '${d.year}-${d.month.toString().padLeft(2, '0')}';

DateTime firstOfMonth(DateTime d) => DateTime(d.year, d.month);

/// Formats digits as "25.000" while typing.
class RupiahInputFormatter extends TextInputFormatter {
  @override
  TextEditingValue formatEditUpdate(TextEditingValue oldValue, TextEditingValue newValue) {
    final value = parseRupiah(newValue.text);
    if (value == null) return const TextEditingValue();
    final text = groupThousands(value);
    return TextEditingValue(
      text: text,
      selection: TextSelection.collapsed(offset: text.length),
    );
  }
}
