import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../config.dart';
import '../util/format.dart';

final apiProvider = Provider<ApiClient>((ref) => ApiClient(baseUrl: AppConfig.apiBaseUrl));

// --- auth ---------------------------------------------------------------------

class AuthState {
  final bool restoring;
  final User? user;
  const AuthState({this.restoring = false, this.user});

  bool get signedIn => user != null;
}

const _tokenKey = 'auth_token';

class AuthController extends StateNotifier<AuthState> {
  final ApiClient _api;

  AuthController(this._api) : super(const AuthState(restoring: true)) {
    _api.onUnauthorized = signOut;
    _restore();
  }

  Future<void> _restore() async {
    final prefs = await SharedPreferences.getInstance();
    final token = prefs.getString(_tokenKey);
    if (token == null) {
      state = const AuthState();
      return;
    }
    _api.token = token;
    try {
      state = AuthState(user: await _api.me());
    } on ApiException catch (e) {
      // Offline: keep the token and try again on next launch; expired: drop it.
      if (e.isUnauthorized) await prefs.remove(_tokenKey);
      _api.token = e.isUnauthorized ? null : token;
      state = const AuthState();
    }
  }

  Future<void> _store(String token, User user) async {
    _api.token = token;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_tokenKey, token);
    state = AuthState(user: user);
  }

  Future<void> signIn(String email, String password) async {
    final (token, user) = await _api.login(email, password);
    await _store(token, user);
  }

  Future<void> register(String email, String name, String password, String invite) async {
    final (token, user) = await _api.register(email, name, password, invite);
    await _store(token, user);
  }

  void updateUser(User user) => state = AuthState(user: user);

  Future<void> signOut() async {
    _api.token = null;
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_tokenKey);
    state = const AuthState();
  }
}

final authProvider = StateNotifierProvider<AuthController, AuthState>(
  (ref) => AuthController(ref.read(apiProvider)),
);

// --- data -----------------------------------------------------------------------

/// The month shown on Home, Activity and Budget (first day of month).
final selectedMonthProvider = StateProvider<DateTime>((ref) => firstOfMonth(DateTime.now()));

final categoriesProvider = FutureProvider<List<Category>>((ref) async {
  ref.watch(authProvider.select((a) => a.user?.id));
  return ref.read(apiProvider).categories();
});

final categoryByIdProvider = Provider<Map<String, Category>>((ref) {
  final cats = ref.watch(categoriesProvider).valueOrNull ?? const [];
  return {for (final c in cats) c.id: c};
});

final summaryProvider = FutureProvider<Summary>((ref) async {
  final month = ref.watch(selectedMonthProvider);
  return ref.read(apiProvider).summary(apiMonth(month));
});

class TxFilter {
  final String kind; // all | out | in | check
  final String search;
  const TxFilter({this.kind = 'all', this.search = ''});

  TxFilter copyWith({String? kind, String? search}) =>
      TxFilter(kind: kind ?? this.kind, search: search ?? this.search);
}

final txFilterProvider = StateProvider<TxFilter>((ref) => const TxFilter());

final transactionsProvider = FutureProvider<List<Transaction>>((ref) async {
  final month = ref.watch(selectedMonthProvider);
  final f = ref.watch(txFilterProvider);
  return ref.read(apiProvider).transactions(
        // "To check" spans all months; the rest is per month.
        month: f.kind == 'check' ? null : apiMonth(month),
        kind: switch (f.kind) { 'out' => 'expense', 'in' => 'income', _ => null },
        needsCheck: f.kind == 'check' ? true : null,
        search: f.search.trim().isEmpty ? null : f.search.trim(),
      );
});

final budgetsProvider = FutureProvider<BudgetList>((ref) async {
  final month = ref.watch(selectedMonthProvider);
  return ref.read(apiProvider).budgets(apiMonth(month));
});

final debtsProvider = FutureProvider<List<Debt>>((ref) async => ref.read(apiProvider).debts());

/// Call after any write so every screen shows fresh numbers.
void refreshData(WidgetRef ref) {
  ref.invalidate(summaryProvider);
  ref.invalidate(transactionsProvider);
  ref.invalidate(budgetsProvider);
  ref.invalidate(debtsProvider);
}
