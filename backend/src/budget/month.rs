//! Monthly budget status: pure function over spending, limits, income and debts.

use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use uuid::Uuid;

use super::rebalance::{self, Cuttable, Rebalance};
use crate::time::{days_in_month, month_start};

/// Spending at or above this share of the limit is "near".
pub const NEAR_PCT: i64 = 85;

#[derive(Debug, Clone)]
pub struct CategoryInput {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub color: String,
    pub flexible: bool,
    pub spent: i64,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DebtInput {
    pub id: Uuid,
    pub name: String,
    pub monthly_payment: i64,
    pub due_day: Option<u32>,
    pub balance: i64,
    pub paid_this_month: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryStatus {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub color: String,
    pub spent: i64,
    pub limit: Option<i64>,
    pub pct: Option<i64>,
    /// ok | near | over | no_budget
    pub state: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DueItem {
    pub debt_id: Uuid,
    pub name: String,
    pub amount: i64,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthStatus {
    pub month: NaiveDate,
    pub income_plan: i64,
    pub total_limit: i64,
    pub spent_budgeted: i64,
    pub spent_total: i64,
    pub remaining: i64,
    pub days_left: u32,
    /// Remaining budget per day for the rest of the month.
    pub per_day_left: i64,
    pub pct_used: Option<i64>,
    /// Installments still to pay this month.
    pub upcoming: Vec<DueItem>,
    pub upcoming_total: i64,
    pub categories: Vec<CategoryStatus>,
    /// Present when limits plus installments exceed the planned income.
    pub suggestion: Option<Rebalance>,
}

fn pct(spent: i64, limit: i64) -> i64 {
    if limit <= 0 {
        return if spent > 0 { 999 } else { 0 };
    }
    ((spent as i128 * 100 + limit as i128 / 2) / limit as i128) as i64
}

fn due_date(month: NaiveDate, day: u32) -> NaiveDate {
    // Day 31 in a 30-day month means the last day.
    let last = days_in_month(month);
    NaiveDate::from_ymd_opt(month.year(), month.month(), day.min(last)).expect("valid day")
}

pub fn compute(
    month: NaiveDate,
    today: NaiveDate,
    income_plan: i64,
    categories: &[CategoryInput],
    debts: &[DebtInput],
) -> MonthStatus {
    let month = month_start(month);
    let dim = days_in_month(month);
    let days_left = if today < month {
        dim
    } else if month_start(today) == month {
        dim - today.day() + 1
    } else {
        0
    };

    let mut statuses: Vec<CategoryStatus> = categories
        .iter()
        .map(|c| {
            let (p, state) = match c.limit {
                None => (None, "no_budget"),
                Some(limit) => {
                    let p = pct(c.spent, limit);
                    let state = if c.spent > limit {
                        "over"
                    } else if p >= NEAR_PCT {
                        "near"
                    } else {
                        "ok"
                    };
                    (Some(p), state)
                }
            };
            CategoryStatus {
                id: c.id,
                slug: c.slug.clone(),
                name: c.name.clone(),
                color: c.color.clone(),
                spent: c.spent,
                limit: c.limit,
                pct: p,
                state,
            }
        })
        .collect();
    // Budgeted categories first, most used first.
    statuses.sort_by(|a, b| {
        b.limit
            .is_some()
            .cmp(&a.limit.is_some())
            .then(b.pct.unwrap_or(-1).cmp(&a.pct.unwrap_or(-1)))
            .then(b.spent.cmp(&a.spent))
    });

    let total_limit: i64 = categories.iter().filter_map(|c| c.limit).sum();
    let spent_budgeted: i64 = categories
        .iter()
        .filter(|c| c.limit.is_some())
        .map(|c| c.spent)
        .sum();
    let spent_total: i64 = categories.iter().map(|c| c.spent).sum();
    let remaining = total_limit - spent_budgeted;
    let per_day_left = if days_left > 0 {
        remaining.max(0) / days_left as i64
    } else {
        0
    };

    let upcoming: Vec<DueItem> = debts
        .iter()
        .filter(|d| d.balance > 0 && d.monthly_payment > 0)
        .filter_map(|d| {
            let owed = d.monthly_payment.min(d.balance) - d.paid_this_month;
            // Overdue but unpaid installments still count.
            if owed <= 0 {
                return None;
            }
            Some(DueItem {
                debt_id: d.id,
                name: d.name.clone(),
                amount: owed,
                due_date: d.due_day.map(|day| due_date(month, day)),
            })
        })
        .collect();
    let upcoming_total: i64 = upcoming.iter().map(|u| u.amount).sum();

    // Installments are covered by the "debt" category's limit if the user budgeted it;
    // otherwise they come on top of the category limits.
    let debt_budgeted = categories
        .iter()
        .any(|c| c.slug == "debt" && c.limit.is_some());
    let committed = total_limit + if debt_budgeted { 0 } else { upcoming_total };
    let suggestion = if income_plan > 0 && committed > income_plan && days_left > 0 {
        let candidates: Vec<Cuttable> = categories
            .iter()
            .filter(|c| c.flexible)
            .filter_map(|c| {
                c.limit.map(|limit| Cuttable {
                    category_id: c.id,
                    name: c.name.clone(),
                    room: (limit - c.spent).max(0),
                })
            })
            .collect();
        Some(rebalance::suggest(committed - income_plan, &candidates))
    } else {
        None
    };

    MonthStatus {
        month,
        income_plan,
        total_limit,
        spent_budgeted,
        spent_total,
        remaining,
        days_left,
        per_day_left,
        pct_used: (total_limit > 0).then(|| pct(spent_budgeted, total_limit)),
        upcoming,
        upcoming_total,
        categories: statuses,
        suggestion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn cat(slug: &str, flexible: bool, spent: i64, limit: Option<i64>) -> CategoryInput {
        CategoryInput {
            id: Uuid::new_v4(),
            slug: slug.into(),
            name: slug.into(),
            color: "grey".into(),
            flexible,
            spent,
            limit,
        }
    }

    fn laptop(paid: i64) -> DebtInput {
        DebtInput {
            id: Uuid::new_v4(),
            name: "Laptop".into(),
            monthly_payment: 750_000,
            due_day: Some(25),
            balance: 3_000_000,
            paid_this_month: paid,
        }
    }

    #[test]
    fn states_and_per_day() {
        let cats = [
            cat("food", true, 845_000, Some(750_000)),
            cat("shopping", true, 300_000, Some(350_000)),
            cat("transport", true, 320_000, Some(600_000)),
            cat("other", true, 50_000, None),
        ];
        let s = compute(d(2026, 10, 1), d(2026, 10, 16), 0, &cats, &[]);
        assert_eq!(s.days_left, 16);
        assert_eq!(s.total_limit, 1_700_000);
        assert_eq!(s.spent_budgeted, 1_465_000);
        assert_eq!(s.spent_total, 1_515_000);
        assert_eq!(s.per_day_left, 235_000 / 16);
        let state = |slug: &str| s.categories.iter().find(|c| c.slug == slug).unwrap().state;
        assert_eq!(state("food"), "over");
        assert_eq!(state("shopping"), "near");
        assert_eq!(state("transport"), "ok");
        assert_eq!(state("other"), "no_budget");
        assert!(s.suggestion.is_none());
    }

    #[test]
    fn installment_over_income_triggers_suggestion() {
        let cats = [
            cat("food", true, 400_000, Some(1_000_000)),
            cat("shopping", true, 100_000, Some(500_000)),
            cat("housing", false, 1_200_000, Some(1_200_000)),
        ];
        // Limits 2.7M + installment 0.75M = 3.45M > income 3.3M → 150k shortfall.
        let s = compute(
            d(2026, 10, 1),
            d(2026, 10, 15),
            3_300_000,
            &cats,
            &[laptop(0)],
        );
        let sug = s.suggestion.expect("suggestion");
        assert_eq!(sug.shortfall, 150_000);
        assert_eq!(sug.uncovered, 0);
        assert!(
            sug.cuts.iter().all(|c| c.name != "housing"),
            "inflexible categories are never cut"
        );
        assert_eq!(s.upcoming_total, 750_000);
        assert_eq!(s.upcoming[0].due_date, Some(d(2026, 10, 25)));
    }

    #[test]
    fn paid_installment_is_not_upcoming() {
        let s = compute(d(2026, 10, 1), d(2026, 10, 26), 0, &[], &[laptop(750_000)]);
        assert!(s.upcoming.is_empty());
    }

    #[test]
    fn budgeted_debt_category_avoids_double_counting() {
        let cats = [
            cat("debt", false, 0, Some(750_000)),
            cat("food", true, 0, Some(1_000_000)),
        ];
        let s = compute(
            d(2026, 10, 1),
            d(2026, 10, 2),
            1_750_000,
            &cats,
            &[laptop(0)],
        );
        assert!(s.suggestion.is_none());
    }

    #[test]
    fn past_and_future_months() {
        let past = compute(d(2026, 9, 1), d(2026, 10, 15), 0, &[], &[]);
        assert_eq!(past.days_left, 0);
        let future = compute(d(2026, 11, 1), d(2026, 10, 15), 0, &[], &[]);
        assert_eq!(future.days_left, 30);
    }

    #[test]
    fn due_day_31_in_short_month() {
        assert_eq!(due_date(d(2026, 11, 1), 31), d(2026, 11, 30));
    }
}
