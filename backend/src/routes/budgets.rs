use std::collections::HashMap;

use axum::Json;
use axum::extract::{Query, State};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::summary::{MonthQuery, month_status};
use super::{check_amount, month_param};
use crate::auth::AuthUser;
use crate::budget::month::MonthStatus;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::time::month_bounds_utc;

#[derive(Serialize)]
pub struct BudgetRow {
    category_id: Uuid,
    slug: String,
    name: String,
    color: String,
    flexible: bool,
    limit: Option<i64>,
    spent: i64,
}

#[derive(Serialize)]
pub struct BudgetList {
    month: NaiveDate,
    items: Vec<BudgetRow>,
    /// When this month has no budgets yet, limits are pre-filled from this month (not saved).
    copied_from: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct BudgetItem {
    category_id: Uuid,
    /// null removes the limit for this category.
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct ReplaceBudgets {
    month: String,
    items: Vec<BudgetItem>,
}

#[derive(Deserialize)]
pub struct ApplyReq {
    month: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Query(q): Query<MonthQuery>,
) -> AppResult<Json<BudgetList>> {
    let month = month_param(q.month.as_deref())?;
    let mut budgets = db::budgets::list(&state.db, uid, month).await?;
    let mut copied_from = None;
    if budgets.is_empty()
        && let Some(prev) = db::budgets::latest_before(&state.db, uid, month).await?
    {
        budgets = db::budgets::list(&state.db, uid, prev).await?;
        copied_from = Some(prev);
    }
    let limits: HashMap<Uuid, i64> = budgets
        .into_iter()
        .map(|b| (b.category_id, b.limit_amount))
        .collect();
    let (from, to) = month_bounds_utc(month);
    let spent: HashMap<Option<Uuid>, i64> =
        db::transactions::expense_by_category(&state.db, uid, from, to)
            .await?
            .into_iter()
            .map(|c| (c.category_id, c.amount))
            .collect();
    let items = db::categories::list(&state.db, uid)
        .await?
        .into_iter()
        .filter(|c| c.kind == "expense")
        .map(|c| BudgetRow {
            limit: limits.get(&c.id).copied(),
            spent: spent.get(&Some(c.id)).copied().unwrap_or(0),
            category_id: c.id,
            slug: c.slug,
            name: c.name,
            color: c.color,
            flexible: c.flexible,
        })
        .collect();
    Ok(Json(BudgetList {
        month,
        items,
        copied_from,
    }))
}

pub async fn replace(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<ReplaceBudgets>,
) -> AppResult<Json<MonthStatus>> {
    let month = month_param(Some(&req.month))?;
    let categories = db::categories::list(&state.db, uid).await?;
    for item in &req.items {
        let cat = categories
            .iter()
            .find(|c| c.id == item.category_id)
            .ok_or_else(|| AppError::bad_request("unknown category"))?;
        if cat.kind != "expense" {
            return Err(AppError::bad_request(
                "budgets are only for expense categories",
            ));
        }
        if let Some(l) = item.limit {
            check_amount("limit", l, true)?;
        }
    }
    let mut tx = state.db.begin().await?;
    for item in &req.items {
        match item.limit {
            Some(l) => db::budgets::upsert(&mut tx, uid, item.category_id, month, l).await?,
            None => db::budgets::delete(&mut tx, uid, item.category_id, month).await?,
        }
    }
    tx.commit().await?;
    Ok(Json(month_status(&state, uid, month).await?))
}

/// Applies the server-computed rebalance. The client only says which month;
/// the numbers are recomputed here so a stale screen can't apply old cuts.
pub async fn apply_suggestion(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<ApplyReq>,
) -> AppResult<Json<MonthStatus>> {
    let month = month_param(req.month.as_deref())?;
    let status = month_status(&state, uid, month).await?;
    let Some(suggestion) = status.suggestion.as_ref().filter(|s| !s.cuts.is_empty()) else {
        return Err(AppError::Conflict(
            "there is no suggestion to apply for this month".into(),
        ));
    };
    let limits: HashMap<Uuid, i64> = db::budgets::list(&state.db, uid, month)
        .await?
        .into_iter()
        .map(|b| (b.category_id, b.limit_amount))
        .collect();
    let mut tx = state.db.begin().await?;
    for cut in &suggestion.cuts {
        let current = limits.get(&cut.category_id).copied().unwrap_or(0);
        db::budgets::upsert(
            &mut tx,
            uid,
            cut.category_id,
            month,
            (current - cut.amount).max(0),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(month_status(&state, uid, month).await?))
}
