use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{check_amount, clean_text};
use crate::auth::AuthUser;
use crate::budget::debt_plan::{self, Plan, Strategy};
use crate::db::{
    self,
    debts::{Debt, NewDebt},
    transactions::{NewTransaction, Transaction},
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::time::{month_bounds_utc, month_start, next_month, now_jakarta};

#[derive(Deserialize)]
pub struct DebtIn {
    name: String,
    #[serde(default = "installment")]
    kind: String,
    creditor: Option<String>,
    principal: i64,
    /// Defaults to the principal for a new debt.
    balance: Option<i64>,
    #[serde(default)]
    monthly_payment: i64,
    /// Annual rate in basis points: 1250 = 12.5%/year.
    #[serde(default)]
    interest_rate_bp: i32,
    due_day: Option<i32>,
}

fn installment() -> String {
    "installment".into()
}

#[derive(Serialize)]
pub struct DebtView {
    #[serde(flatten)]
    debt: Debt,
    paid_this_month: i64,
}

#[derive(Deserialize)]
pub struct PayReq {
    amount: i64,
    paid_at: Option<DateTime<FixedOffset>>,
    /// Also record the payment as an expense in the "Debt & installments" category.
    #[serde(default = "yes")]
    record_transaction: bool,
}

fn yes() -> bool {
    true
}

#[derive(Serialize)]
pub struct PayResp {
    debt: Debt,
    payment_id: Uuid,
    transaction: Option<Transaction>,
}

#[derive(Deserialize)]
pub struct PlanQuery {
    strategy: Option<Strategy>,
    #[serde(default)]
    extra: i64,
}

#[derive(Serialize)]
pub struct PlanResp {
    #[serde(flatten)]
    plan: Plan,
    /// Calendar month (YYYY-MM-01) each debt is paid off; month 1 = this month.
    payoff_dates: HashMap<Uuid, NaiveDate>,
    finish_date: Option<NaiveDate>,
}

fn validate(d: &DebtIn) -> AppResult<(String, i64)> {
    let name = clean_text("name", &d.name, 80)?;
    if d.kind != "installment" && d.kind != "loan" {
        return Err(AppError::bad_request("kind must be installment or loan"));
    }
    check_amount("principal", d.principal, false)?;
    let balance = d.balance.unwrap_or(d.principal);
    check_amount("balance", balance, true)?;
    check_amount("monthly_payment", d.monthly_payment, true)?;
    if !(0..=100_000).contains(&d.interest_rate_bp) {
        return Err(AppError::bad_request(
            "interest_rate_bp must be between 0 and 100000",
        ));
    }
    if d.due_day.is_some_and(|day| !(1..=31).contains(&day)) {
        return Err(AppError::bad_request("due_day must be between 1 and 31"));
    }
    Ok((name, balance))
}

fn add_months(month: NaiveDate, n: u32) -> NaiveDate {
    (0..n).fold(month_start(month), |m, _| next_month(m))
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
) -> AppResult<Json<Vec<DebtView>>> {
    let (from, to) = month_bounds_utc(month_start(now_jakarta().date_naive()));
    let paid: HashMap<Uuid, i64> = db::debts::paid_between(&state.db, uid, from, to)
        .await?
        .into_iter()
        .collect();
    let debts = db::debts::list(&state.db, uid).await?;
    Ok(Json(
        debts
            .into_iter()
            .map(|d| DebtView {
                paid_this_month: paid.get(&d.id).copied().unwrap_or(0),
                debt: d,
            })
            .collect(),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<DebtIn>,
) -> AppResult<(StatusCode, Json<Debt>)> {
    let (name, balance) = validate(&req)?;
    let creditor = req
        .creditor
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    let debt = db::debts::create(
        &state.db,
        uid,
        NewDebt {
            name: &name,
            kind: &req.kind,
            creditor,
            principal: req.principal,
            balance,
            monthly_payment: req.monthly_payment,
            interest_rate_bp: req.interest_rate_bp,
            due_day: req.due_day,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(debt)))
}

pub async fn update(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<DebtIn>,
) -> AppResult<Json<Debt>> {
    let (name, balance) = validate(&req)?;
    let creditor = req
        .creditor
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    db::debts::update(
        &state.db,
        uid,
        id,
        NewDebt {
            name: &name,
            kind: &req.kind,
            creditor,
            principal: req.principal,
            balance,
            monthly_payment: req.monthly_payment,
            interest_rate_bp: req.interest_rate_bp,
            due_day: req.due_day,
        },
    )
    .await?
    .map(Json)
    .ok_or(AppError::NotFound)
}

pub async fn remove(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if db::debts::delete(&state.db, uid, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn pay(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PayReq>,
) -> AppResult<(StatusCode, Json<PayResp>)> {
    check_amount("amount", req.amount, false)?;
    let paid_at = req.paid_at.unwrap_or_else(now_jakarta).with_timezone(&Utc);
    let category = if req.record_transaction {
        db::categories::find_by_slug(&state.db, uid, "debt").await?
    } else {
        None
    };

    let mut tx = state.db.begin().await?;
    let mut debt = db::debts::get_for_update(&mut tx, uid, id)
        .await?
        .ok_or(AppError::NotFound)?;
    if req.amount > debt.balance {
        return Err(AppError::bad_request(format!(
            "payment is larger than the remaining balance ({})",
            debt.balance
        )));
    }
    let transaction = if req.record_transaction {
        Some(
            db::transactions::insert(
                &mut tx,
                NewTransaction {
                    user_id: uid,
                    category_id: category.map(|c| c.id),
                    kind: "expense",
                    amount: req.amount,
                    merchant: &debt.name,
                    note: debt.creditor.as_deref(),
                    occurred_at: paid_at,
                    source: "debt_payment",
                    receipt_id: None,
                    items: serde_json::json!([]),
                    needs_check: false,
                },
            )
            .await?,
        )
    } else {
        None
    };
    let payment_id = db::debts::insert_payment(
        &mut tx,
        id,
        uid,
        req.amount,
        paid_at,
        transaction.as_ref().map(|t| t.id),
    )
    .await?;
    debt.balance -= req.amount;
    db::debts::set_balance(&mut tx, id, debt.balance).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(PayResp {
            debt,
            payment_id,
            transaction,
        }),
    ))
}

pub async fn plan(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Query(q): Query<PlanQuery>,
) -> AppResult<Json<PlanResp>> {
    check_amount("extra", q.extra, true)?;
    let debts: Vec<debt_plan::DebtInput> = db::debts::list(&state.db, uid)
        .await?
        .into_iter()
        .map(|d| debt_plan::DebtInput {
            id: d.id,
            name: d.name,
            balance: d.balance,
            interest_rate_bp: d.interest_rate_bp,
            min_payment: d.monthly_payment,
        })
        .collect();
    let plan = debt_plan::simulate(&debts, q.extra, q.strategy.unwrap_or(Strategy::Avalanche));
    let this_month = now_jakarta().date_naive();
    let payoff_dates = plan
        .debts
        .iter()
        .filter_map(|d| {
            d.payoff_month
                .map(|m| (d.id, add_months(this_month, m - 1)))
        })
        .collect();
    let finish_date = plan
        .months
        .map(|m| add_months(this_month, m.saturating_sub(1)));
    debug_assert!(finish_date.is_none_or(|d| d.day() == 1));
    Ok(Json(PlanResp {
        plan,
        payoff_dates,
        finish_date,
    }))
}
