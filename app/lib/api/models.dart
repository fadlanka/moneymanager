// Plain data classes mirroring the backend JSON. Money is always `int` rupiah.

int _int(dynamic v) => v == null ? 0 : (v as num).toInt();
int? _intOrNull(dynamic v) => v == null ? null : (v as num).toInt();
double _double(dynamic v) => v == null ? 0 : (v as num).toDouble();
DateTime _date(dynamic v) => DateTime.parse(v as String);
DateTime? _dateOrNull(dynamic v) => v == null ? null : DateTime.parse(v as String);
List<Map<String, dynamic>> _list(dynamic v) =>
    v == null ? const [] : (v as List).cast<Map<String, dynamic>>();

class User {
  final String id;
  final String email;
  final String name;
  final int monthlyIncome;

  User({required this.id, required this.email, required this.name, required this.monthlyIncome});

  factory User.fromJson(Map<String, dynamic> j) => User(
        id: j['id'] as String,
        email: j['email'] as String,
        name: j['name'] as String,
        monthlyIncome: _int(j['monthly_income']),
      );

  String get initials {
    final parts = name.trim().split(RegExp(r'\s+')).where((p) => p.isNotEmpty).toList();
    if (parts.isEmpty) return '?';
    if (parts.length == 1) return parts.first.substring(0, 1).toUpperCase();
    return (parts[0].substring(0, 1) + parts[1].substring(0, 1)).toUpperCase();
  }
}

class Category {
  final String id;
  final String slug;
  final String name;
  final String kind;
  final String color;
  final bool flexible;

  Category({
    required this.id,
    required this.slug,
    required this.name,
    required this.kind,
    required this.color,
    required this.flexible,
  });

  factory Category.fromJson(Map<String, dynamic> j) => Category(
        id: j['id'] as String,
        slug: j['slug'] as String,
        name: j['name'] as String,
        kind: j['kind'] as String,
        color: j['color'] as String,
        flexible: j['flexible'] as bool? ?? true,
      );

  bool get isIncome => kind == 'income';
}

class LineItem {
  String name;
  double qty;
  int? unitPrice;
  int total;
  double confidence;
  bool flagged;
  bool included;

  LineItem({
    required this.name,
    this.qty = 1,
    this.unitPrice,
    required this.total,
    this.confidence = 1,
    this.flagged = false,
    this.included = true,
  });

  factory LineItem.fromJson(Map<String, dynamic> j) => LineItem(
        name: j['name'] as String? ?? '',
        qty: j['qty'] == null ? 1 : _double(j['qty']),
        unitPrice: _intOrNull(j['unit_price']),
        total: _int(j['total']),
        confidence: _double(j['confidence']),
        flagged: j['flagged'] as bool? ?? false,
      );

  Map<String, dynamic> toJson() => {
        'name': name,
        'qty': qty,
        'unit_price': unitPrice,
        'total': total,
      };
}

class Transaction {
  final String id;
  final String? categoryId;
  final String kind;
  final int amount;
  final String merchant;
  final String? note;
  final DateTime occurredAt;
  final String source;
  final String? receiptId;
  final List<LineItem> items;
  final bool needsCheck;

  Transaction({
    required this.id,
    required this.categoryId,
    required this.kind,
    required this.amount,
    required this.merchant,
    required this.note,
    required this.occurredAt,
    required this.source,
    required this.receiptId,
    required this.items,
    required this.needsCheck,
  });

  factory Transaction.fromJson(Map<String, dynamic> j) => Transaction(
        id: j['id'] as String,
        categoryId: j['category_id'] as String?,
        kind: j['kind'] as String,
        amount: _int(j['amount']),
        merchant: j['merchant'] as String,
        note: j['note'] as String?,
        occurredAt: _date(j['occurred_at']),
        source: j['source'] as String,
        receiptId: j['receipt_id'] as String?,
        items: _list(j['items']).map(LineItem.fromJson).toList(),
        needsCheck: j['needs_check'] as bool? ?? false,
      );

  bool get isIncome => kind == 'income';
  bool get fromScan => source == 'scan';
}

class Issue {
  final String field;
  final String code;
  final String message;

  Issue({required this.field, required this.code, required this.message});

  factory Issue.fromJson(Map<String, dynamic> j) => Issue(
        field: j['field'] as String,
        code: j['code'] as String,
        message: j['message'] as String,
      );
}

/// What the AI read, after the backend's validation.
class Extraction {
  final bool isReceipt;
  final String? merchant;
  final double merchantConfidence;
  final DateTime? date;
  final double dateConfidence;
  final List<LineItem> items;
  final int? total;
  final double totalConfidence;
  final int? tax;
  final int? discount;
  final int? service;

  Extraction({
    required this.isReceipt,
    required this.merchant,
    required this.merchantConfidence,
    required this.date,
    required this.dateConfidence,
    required this.items,
    required this.total,
    required this.totalConfidence,
    required this.tax,
    required this.discount,
    required this.service,
  });

  factory Extraction.fromJson(Map<String, dynamic> j) {
    Map<String, dynamic> f(String k) => (j[k] as Map<String, dynamic>?) ?? const {};
    return Extraction(
      isReceipt: j['is_receipt'] as bool? ?? true,
      merchant: f('merchant')['value'] as String?,
      merchantConfidence: _double(f('merchant')['confidence']),
      date: _dateOrNull(f('date')['value']),
      dateConfidence: _double(f('date')['confidence']),
      items: _list(j['items']).map(LineItem.fromJson).toList(),
      total: _intOrNull(f('total')['value']),
      totalConfidence: _double(f('total')['confidence']),
      tax: _intOrNull(f('tax')['value']),
      discount: _intOrNull(f('discount')['value']),
      service: _intOrNull(f('service')['value']),
    );
  }
}

class Receipt {
  final String id;
  final String status;
  final Extraction? extraction;
  final List<Issue> issues;
  final String? error;
  final String? suggestedCategoryId;
  final String? provider;
  final String? model;
  final int? latencyMs;

  Receipt({
    required this.id,
    required this.status,
    required this.extraction,
    required this.issues,
    required this.error,
    required this.suggestedCategoryId,
    required this.provider,
    required this.model,
    required this.latencyMs,
  });

  factory Receipt.fromJson(Map<String, dynamic> j) => Receipt(
        id: j['id'] as String,
        status: j['status'] as String,
        extraction: j['extraction'] == null
            ? null
            : Extraction.fromJson(j['extraction'] as Map<String, dynamic>),
        issues: _list(j['issues']).map(Issue.fromJson).toList(),
        error: j['error'] as String?,
        suggestedCategoryId: j['suggested_category_id'] as String?,
        provider: j['provider'] as String?,
        model: j['model'] as String?,
        latencyMs: _intOrNull(j['latency_ms']),
      );

  bool get isDone => status == 'ready' || status == 'needs_review';
  bool get isFailed => status == 'failed';
  bool get isWaiting => status == 'pending' || status == 'processing';
}

class CategoryStatus {
  final String id;
  final String slug;
  final String name;
  final String color;
  final int spent;
  final int? limit;
  final int? pct;
  final String state;

  CategoryStatus({
    required this.id,
    required this.slug,
    required this.name,
    required this.color,
    required this.spent,
    required this.limit,
    required this.pct,
    required this.state,
  });

  factory CategoryStatus.fromJson(Map<String, dynamic> j) => CategoryStatus(
        id: j['id'] as String,
        slug: j['slug'] as String,
        name: j['name'] as String,
        color: j['color'] as String,
        spent: _int(j['spent']),
        limit: _intOrNull(j['limit']),
        pct: _intOrNull(j['pct']),
        state: j['state'] as String,
      );

  bool get isOver => state == 'over';
  bool get isNear => state == 'near';
}

class DueItem {
  final String debtId;
  final String name;
  final int amount;
  final DateTime? dueDate;

  DueItem({required this.debtId, required this.name, required this.amount, required this.dueDate});

  factory DueItem.fromJson(Map<String, dynamic> j) => DueItem(
        debtId: j['debt_id'] as String,
        name: j['name'] as String,
        amount: _int(j['amount']),
        dueDate: _dateOrNull(j['due_date']),
      );
}

class Cut {
  final String categoryId;
  final String name;
  final int amount;

  Cut({required this.categoryId, required this.name, required this.amount});

  factory Cut.fromJson(Map<String, dynamic> j) => Cut(
        categoryId: j['category_id'] as String,
        name: j['name'] as String,
        amount: _int(j['amount']),
      );
}

class Rebalance {
  final int shortfall;
  final List<Cut> cuts;
  final int uncovered;

  Rebalance({required this.shortfall, required this.cuts, required this.uncovered});

  factory Rebalance.fromJson(Map<String, dynamic> j) => Rebalance(
        shortfall: _int(j['shortfall']),
        cuts: _list(j['cuts']).map(Cut.fromJson).toList(),
        uncovered: _int(j['uncovered']),
      );
}

class MonthStatus {
  final DateTime month;
  final int incomePlan;
  final int totalLimit;
  final int spentBudgeted;
  final int spentTotal;
  final int remaining;
  final int daysLeft;
  final int perDayLeft;
  final int? pctUsed;
  final List<DueItem> upcoming;
  final int upcomingTotal;
  final List<CategoryStatus> categories;
  final Rebalance? suggestion;

  MonthStatus({
    required this.month,
    required this.incomePlan,
    required this.totalLimit,
    required this.spentBudgeted,
    required this.spentTotal,
    required this.remaining,
    required this.daysLeft,
    required this.perDayLeft,
    required this.pctUsed,
    required this.upcoming,
    required this.upcomingTotal,
    required this.categories,
    required this.suggestion,
  });

  factory MonthStatus.fromJson(Map<String, dynamic> j) => MonthStatus(
        month: _date(j['month']),
        incomePlan: _int(j['income_plan']),
        totalLimit: _int(j['total_limit']),
        spentBudgeted: _int(j['spent_budgeted']),
        spentTotal: _int(j['spent_total']),
        remaining: _int(j['remaining']),
        daysLeft: _int(j['days_left']),
        perDayLeft: _int(j['per_day_left']),
        pctUsed: _intOrNull(j['pct_used']),
        upcoming: _list(j['upcoming']).map(DueItem.fromJson).toList(),
        upcomingTotal: _int(j['upcoming_total']),
        categories: _list(j['categories']).map(CategoryStatus.fromJson).toList(),
        suggestion: j['suggestion'] == null
            ? null
            : Rebalance.fromJson(j['suggestion'] as Map<String, dynamic>),
      );

  List<CategoryStatus> get budgeted => categories.where((c) => c.limit != null).toList();
}

class DailyTotal {
  final DateTime day;
  final int amount;

  DailyTotal({required this.day, required this.amount});

  factory DailyTotal.fromJson(Map<String, dynamic> j) =>
      DailyTotal(day: _date(j['day']), amount: _int(j['amount']));
}

class Summary {
  final DateTime month;
  final int income;
  final int expense;
  final int net;
  final int needsCheckCount;
  final MonthStatus budget;
  final List<Transaction> recent;
  final List<DailyTotal> daily;

  Summary({
    required this.month,
    required this.income,
    required this.expense,
    required this.net,
    required this.needsCheckCount,
    required this.budget,
    required this.recent,
    required this.daily,
  });

  factory Summary.fromJson(Map<String, dynamic> j) => Summary(
        month: _date(j['month']),
        income: _int(j['income']),
        expense: _int(j['expense']),
        net: _int(j['net']),
        needsCheckCount: _int(j['needs_check_count']),
        budget: MonthStatus.fromJson(j['budget'] as Map<String, dynamic>),
        recent: _list(j['recent']).map(Transaction.fromJson).toList(),
        daily: _list(j['daily']).map(DailyTotal.fromJson).toList(),
      );
}

class BudgetRow {
  final String categoryId;
  final String slug;
  final String name;
  final String color;
  final int? limit;
  final int spent;

  BudgetRow({
    required this.categoryId,
    required this.slug,
    required this.name,
    required this.color,
    required this.limit,
    required this.spent,
  });

  factory BudgetRow.fromJson(Map<String, dynamic> j) => BudgetRow(
        categoryId: j['category_id'] as String,
        slug: j['slug'] as String,
        name: j['name'] as String,
        color: j['color'] as String,
        limit: _intOrNull(j['limit']),
        spent: _int(j['spent']),
      );
}

class BudgetList {
  final DateTime month;
  final List<BudgetRow> items;
  final DateTime? copiedFrom;

  BudgetList({required this.month, required this.items, required this.copiedFrom});

  factory BudgetList.fromJson(Map<String, dynamic> j) => BudgetList(
        month: _date(j['month']),
        items: _list(j['items']).map(BudgetRow.fromJson).toList(),
        copiedFrom: _dateOrNull(j['copied_from']),
      );
}

class Debt {
  final String id;
  final String name;
  final String kind;
  final String? creditor;
  final int principal;
  final int balance;
  final int monthlyPayment;
  final int interestRateBp;
  final int? dueDay;
  final int paidThisMonth;

  Debt({
    required this.id,
    required this.name,
    required this.kind,
    required this.creditor,
    required this.principal,
    required this.balance,
    required this.monthlyPayment,
    required this.interestRateBp,
    required this.dueDay,
    required this.paidThisMonth,
  });

  factory Debt.fromJson(Map<String, dynamic> j) => Debt(
        id: j['id'] as String,
        name: j['name'] as String,
        kind: j['kind'] as String,
        creditor: j['creditor'] as String?,
        principal: _int(j['principal']),
        balance: _int(j['balance']),
        monthlyPayment: _int(j['monthly_payment']),
        interestRateBp: _int(j['interest_rate_bp']),
        dueDay: _intOrNull(j['due_day']),
        paidThisMonth: _int(j['paid_this_month']),
      );

  int get paid => principal - balance;
  double get progress => principal == 0 ? 0 : (paid / principal).clamp(0, 1).toDouble();
  bool get settled => balance == 0;
}

class DebtPayoff {
  final String id;
  final String name;
  final int? payoffMonth;
  final int interestPaid;
  final int priority;

  DebtPayoff({
    required this.id,
    required this.name,
    required this.payoffMonth,
    required this.interestPaid,
    required this.priority,
  });

  factory DebtPayoff.fromJson(Map<String, dynamic> j) => DebtPayoff(
        id: j['id'] as String,
        name: j['name'] as String,
        payoffMonth: _intOrNull(j['payoff_month']),
        interestPaid: _int(j['interest_paid']),
        priority: _int(j['priority']),
      );
}

class DebtPlan {
  final String strategy;
  final int monthlyBudget;
  final int? months;
  final int totalInterest;
  final bool feasible;
  final List<DebtPayoff> debts;
  final Map<String, DateTime> payoffDates;
  final DateTime? finishDate;

  DebtPlan({
    required this.strategy,
    required this.monthlyBudget,
    required this.months,
    required this.totalInterest,
    required this.feasible,
    required this.debts,
    required this.payoffDates,
    required this.finishDate,
  });

  factory DebtPlan.fromJson(Map<String, dynamic> j) => DebtPlan(
        strategy: j['strategy'] as String,
        monthlyBudget: _int(j['monthly_budget']),
        months: _intOrNull(j['months']),
        totalInterest: _int(j['total_interest']),
        feasible: j['feasible'] as bool? ?? false,
        debts: _list(j['debts']).map(DebtPayoff.fromJson).toList(),
        payoffDates: ((j['payoff_dates'] as Map<String, dynamic>?) ?? const {})
            .map((k, v) => MapEntry(k, DateTime.parse(v as String))),
        finishDate: _dateOrNull(j['finish_date']),
      );
}
