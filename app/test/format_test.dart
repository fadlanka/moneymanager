import 'package:flutter_test/flutter_test.dart';
import 'package:moneymanager/util/format.dart';

void main() {
  test('rupiah formatting', () {
    expect(groupThousands(0), '0');
    expect(groupThousands(87400), '87.400');
    expect(groupThousands(4250000), '4.250.000');
    expect(rupiah(-38000), '−Rp 38.000');
    expect(signedAmount(1500000, income: true), '+1.500.000');
  });

  test('parsing typed amounts', () {
    expect(parseRupiah('Rp 25.000'), 25000);
    expect(parseRupiah(''), null);
    expect(parseRupiah('1.250.000'), 1250000);
  });

  test('api month', () {
    expect(apiMonth(DateTime(2026, 3, 9)), '2026-03');
  });
}
