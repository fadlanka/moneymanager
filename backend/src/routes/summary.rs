use std::collections::HashMap;

use axum::Json;
use axum::extract::{Query, State};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::month_param;
use crate::auth::AuthUser;
use crate::budget::month::{self, CategoryInput, DebtInput, MonthStatus};
use crate::db::{
    self,
    transactions::{DailyTotal, ListFilter, Transaction},
};
use crate::error::AppResult;
use crate::state::AppState;
use crate::time::{month_bounds_utc, now_jakarta};

#[derive(Deserialize)]
pub struct MonthQuery {
    pub month: Option<String>,
}

#[derive(Serialize)]
pub struct Summary {
    month: NaiveDate,
    income: i64,
    expense: i64,
    net: i64,
    needs_check_count: i64,
    budget: MonthStatus,
    recent: Vec<Transaction>,
    daily: Vec<DailyTotal>,
}

/// Gathers spending, limits, income plan and debts, then runs the pure calculator.
pub(crate) async fn month_status(
    state: &AppState,
    uid: Uuid,
    month: NaiveDate,
) -> AppResult<MonthStatus> {
    let (from, to) = month_bounds_utc(month);
    let user = db::users::get(&state.db, uid).await?;
    let categories = db::categories::list(&state.db, uid).await?;
    let spent: HashMap<Option<Uuid>, i64> =
        db::transactions::expense_by_category(&state.db, uid, from, to)
            .await?
            .into_iter()
            .map(|c| (c.category_id, c.amount))
            .collect();
    let limits: HashMap<Uuid, i64> = db::budgets::list(&state.db, uid, month)
        .await?
        .into_iter()
        .map(|b| (b.category_id, b.limit_amount))
        .collect();

    let mut inputs: Vec<CategoryInput> = categories
        .iter()
        .filter(|c| c.kind == "expense")
        .map(|c| CategoryInput {
            id: c.id,
            slug: c.slug.clone(),
            name: c.name.clone(),
            color: c.color.clone(),
            flexible: c.flexible,
            spent: spent.get(&Some(c.id)).copied().unwrap_or(0),
            limit: limits.get(&c.id).copied(),
        })
        .collect();
    // Expenses without a category still count toward the month's spending.
    if let Some(&uncategorized) = spent.get(&None) {
        inputs.push(CategoryInput {
            id: Uuid::nil(),
            slug: "uncategorized".into(),
            name: "Uncategorized".into(),
            color: "grey".into(),
            flexible: false,
            spent: uncategorized,
            limit: None,
        });
    }

    let paid: HashMap<Uuid, i64> = db::debts::paid_between(&state.db, uid, from, to)
        .await?
        .into_iter()
        .collect();
    let debts: Vec<DebtInput> = db::debts::list(&state.db, uid)
        .await?
        .into_iter()
        .map(|d| DebtInput {
            id: d.id,
            name: d.name,
            monthly_payment: d.monthly_payment,
            due_day: d.due_day.map(|v| v as u32),
            balance: d.balance,
            paid_this_month: paid.get(&d.id).copied().unwrap_or(0),
        })
        .collect();

    Ok(month::compute(
        month,
        now_jakarta().date_naive(),
        user.monthly_income,
        &inputs,
        &debts,
    ))
}

pub async fn summary(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Query(q): Query<MonthQuery>,
) -> AppResult<Json<Summary>> {
    let month = month_param(q.month.as_deref())?;
    let (from, to) = month_bounds_utc(month);
    let (income, expense) = db::transactions::totals(&state.db, uid, from, to).await?;
    let recent = db::transactions::list(
        &state.db,
        uid,
        &ListFilter {
            from: Some(from),
            to: Some(to),
            limit: 5,
            ..Default::default()
        },
    )
    .await?;
    Ok(Json(Summary {
        month,
        income,
        expense,
        net: income - expense,
        needs_check_count: db::transactions::count_needs_check(&state.db, uid).await?,
        budget: month_status(&state, uid, month).await?,
        recent,
        daily: db::transactions::daily_expense(&state.db, uid, from, to).await?,
    }))
}
