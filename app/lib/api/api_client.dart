import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;

import 'models.dart';

class ApiException implements Exception {
  final int status;
  final String message;
  ApiException(this.status, this.message);

  bool get isUnauthorized => status == 401;
  bool get isConflict => status == 409;

  @override
  String toString() => message;
}

/// Thin HTTP client. One method per endpoint; no state besides the token.
class ApiClient {
  final String baseUrl;
  final http.Client _http;
  String? token;

  /// Called on any 401 so the app can drop back to the sign-in screen.
  void Function()? onUnauthorized;

  ApiClient({required this.baseUrl, http.Client? client}) : _http = client ?? http.Client();

  Uri _uri(String path, [Map<String, String?>? query]) {
    final q = <String, String>{};
    query?.forEach((k, v) {
      if (v != null && v.isNotEmpty) q[k] = v;
    });
    return Uri.parse('$baseUrl$path').replace(queryParameters: q.isEmpty ? null : q);
  }

  Map<String, String> get _headers => {
        'Content-Type': 'application/json',
        if (token != null) 'Authorization': 'Bearer $token',
      };

  Future<dynamic> _send(String method, String path, {Object? body, Map<String, String?>? query}) async {
    final req = http.Request(method, _uri(path, query))..headers.addAll(_headers);
    if (body != null) req.body = jsonEncode(body);
    final http.Response resp;
    try {
      resp = await http.Response.fromStream(await _http.send(req));
    } catch (_) {
      throw ApiException(0, 'Can\'t reach the server. Check your connection.');
    }
    return _decode(resp);
  }

  dynamic _decode(http.Response resp) {
    final text = utf8.decode(resp.bodyBytes);
    final json = text.isEmpty ? null : jsonDecode(text);
    if (resp.statusCode >= 200 && resp.statusCode < 300) return json;
    if (resp.statusCode == 401 && token != null) onUnauthorized?.call();
    final message = (json is Map && json['error'] is Map)
        ? (json['error']['message'] as String? ?? 'Request failed')
        : (text.isEmpty ? 'Request failed (${resp.statusCode})' : text);
    throw ApiException(resp.statusCode, message);
  }

  // --- auth -----------------------------------------------------------------

  Future<(String, User)> login(String email, String password) async {
    final j = await _send('POST', '/auth/login', body: {'email': email, 'password': password});
    return (j['token'] as String, User.fromJson(j['user'] as Map<String, dynamic>));
  }

  Future<(String, User)> register(String email, String name, String password, String inviteCode) async {
    final j = await _send('POST', '/auth/register', body: {
      'email': email,
      'name': name,
      'password': password,
      'invite_code': inviteCode,
    });
    return (j['token'] as String, User.fromJson(j['user'] as Map<String, dynamic>));
  }

  Future<User> me() async => User.fromJson(await _send('GET', '/me') as Map<String, dynamic>);

  Future<User> updateMe({String? name, int? monthlyIncome}) async => User.fromJson(
        await _send('PATCH', '/me', body: {
          if (name != null) 'name': name,
          if (monthlyIncome != null) 'monthly_income': monthlyIncome,
        }) as Map<String, dynamic>,
      );

  // --- categories -------------------------------------------------------------

  Future<List<Category>> categories() async =>
      ((await _send('GET', '/categories')) as List).cast<Map<String, dynamic>>().map(Category.fromJson).toList();

  // --- receipts ---------------------------------------------------------------

  /// Returns the new receipt id. Works on web too (bytes, not dart:io File).
  Future<String> uploadReceipt(Uint8List bytes, String filename) async {
    final req = http.MultipartRequest('POST', _uri('/receipts'));
    if (token != null) req.headers['Authorization'] = 'Bearer $token';
    req.files.add(http.MultipartFile.fromBytes('image', bytes, filename: filename));
    final http.Response resp;
    try {
      resp = await http.Response.fromStream(await _http.send(req));
    } catch (_) {
      throw ApiException(0, 'Can\'t reach the server. Check your connection.');
    }
    final j = _decode(resp) as Map<String, dynamic>;
    return j['id'] as String;
  }

  Future<Receipt> receipt(String id) async =>
      Receipt.fromJson(await _send('GET', '/receipts/$id') as Map<String, dynamic>);

  Future<void> retryReceipt(String id) async => _send('POST', '/receipts/$id/retry');

  // --- transactions -----------------------------------------------------------

  Future<List<Transaction>> transactions({
    String? month,
    String? kind,
    bool? needsCheck,
    String? search,
    String? categoryId,
  }) async {
    final j = await _send('GET', '/transactions', query: {
      'month': month,
      'kind': kind,
      'needs_check': needsCheck?.toString(),
      'q': search,
      'category_id': categoryId,
    });
    return (j as List).cast<Map<String, dynamic>>().map(Transaction.fromJson).toList();
  }

  Future<Transaction> createTransaction(Map<String, dynamic> body) async =>
      Transaction.fromJson(await _send('POST', '/transactions', body: body) as Map<String, dynamic>);

  Future<Transaction> updateTransaction(String id, Map<String, dynamic> body) async =>
      Transaction.fromJson(await _send('PATCH', '/transactions/$id', body: body) as Map<String, dynamic>);

  Future<void> deleteTransaction(String id) async => _send('DELETE', '/transactions/$id');

  // --- summary & budgets ------------------------------------------------------

  Future<Summary> summary(String month) async =>
      Summary.fromJson(await _send('GET', '/summary', query: {'month': month}) as Map<String, dynamic>);

  Future<BudgetList> budgets(String month) async =>
      BudgetList.fromJson(await _send('GET', '/budgets', query: {'month': month}) as Map<String, dynamic>);

  /// `limits`: category id -> limit (null removes the limit).
  Future<MonthStatus> saveBudgets(String month, Map<String, int?> limits) async => MonthStatus.fromJson(
        await _send('PUT', '/budgets', body: {
          'month': month,
          'items': limits.entries.map((e) => {'category_id': e.key, 'limit': e.value}).toList(),
        }) as Map<String, dynamic>,
      );

  Future<MonthStatus> applySuggestion(String month) async => MonthStatus.fromJson(
        await _send('POST', '/budgets/apply-suggestion', body: {'month': month}) as Map<String, dynamic>,
      );

  // --- debts ------------------------------------------------------------------

  Future<List<Debt>> debts() async =>
      ((await _send('GET', '/debts')) as List).cast<Map<String, dynamic>>().map(Debt.fromJson).toList();

  Future<void> createDebt(Map<String, dynamic> body) async => _send('POST', '/debts', body: body);

  Future<void> updateDebt(String id, Map<String, dynamic> body) async => _send('PUT', '/debts/$id', body: body);

  Future<void> deleteDebt(String id) async => _send('DELETE', '/debts/$id');

  Future<void> payDebt(String id, int amount) async =>
      _send('POST', '/debts/$id/payments', body: {'amount': amount, 'record_transaction': true});

  Future<DebtPlan> debtPlan(String strategy, int extra) async => DebtPlan.fromJson(
        await _send('GET', '/debts/plan', query: {'strategy': strategy, 'extra': '$extra'})
            as Map<String, dynamic>,
      );
}
