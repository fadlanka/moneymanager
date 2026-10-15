use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{check_amount, clean_text, month_param};
use crate::auth::AuthUser;
use crate::db::{
    self,
    transactions::{ListFilter, NewTransaction, Patch, Transaction},
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::time::{jakarta, month_bounds_utc, now_jakarta};

#[derive(Deserialize)]
pub struct ListQuery {
    month: Option<String>,
    category_id: Option<Uuid>,
    kind: Option<String>,
    needs_check: Option<bool>,
    q: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemIn {
    name: String,
    #[serde(default = "one")]
    qty: f64,
    unit_price: Option<i64>,
    total: i64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
pub struct CreateTx {
    #[serde(default = "expense")]
    kind: String,
    amount: i64,
    merchant: String,
    note: Option<String>,
    occurred_at: Option<DateTime<FixedOffset>>,
    category_id: Option<Uuid>,
    #[serde(default)]
    items: Vec<ItemIn>,
    /// Set when saving the result of a scanned receipt.
    receipt_id: Option<Uuid>,
    /// "Save now, check later". Shows up under the "To check" filter.
    #[serde(default)]
    needs_check: bool,
}

fn expense() -> String {
    "expense".into()
}

#[derive(Deserialize)]
pub struct UpdateTx {
    amount: Option<i64>,
    merchant: Option<String>,
    // Distinguish "absent" from explicit null for clearable fields.
    #[serde(default, deserialize_with = "double_option")]
    note: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    category_id: Option<Option<Uuid>>,
    occurred_at: Option<DateTime<FixedOffset>>,
    needs_check: Option<bool>,
}

fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Transaction>>> {
    let (from, to) = match q.month.as_deref() {
        Some(_) => {
            let (f, t) = month_bounds_utc(month_param(q.month.as_deref())?);
            (Some(f), Some(t))
        }
        None => (None, None),
    };
    if let Some(k) = &q.kind
        && k != "expense"
        && k != "income"
    {
        return Err(AppError::bad_request("kind must be expense or income"));
    }
    let filter = ListFilter {
        from,
        to,
        category_id: q.category_id,
        kind: q.kind,
        needs_check: q.needs_check,
        search: q.q.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        limit: q.limit.unwrap_or(200).clamp(1, 500),
    };
    Ok(Json(db::transactions::list(&state.db, uid, &filter).await?))
}

pub async fn get_one(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Transaction>> {
    db::transactions::get(&state.db, uid, id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn check_category(
    state: &AppState,
    uid: Uuid,
    id: Option<Uuid>,
    kind: &str,
) -> AppResult<Option<db::categories::Category>> {
    let Some(id) = id else { return Ok(None) };
    let cat = db::categories::get(&state.db, uid, id)
        .await?
        .ok_or_else(|| AppError::bad_request("unknown category"))?;
    if cat.kind != kind {
        return Err(AppError::bad_request(format!(
            "category {} is not an {kind} category",
            cat.name
        )));
    }
    Ok(Some(cat))
}

/// Which fields the user changed compared to the AI extraction.
/// Saved with the receipt; this diff is the raw material for evaluation.
fn corrected_fields(ai: &Value, saved: &Value, chosen_slug: Option<&str>) -> Vec<String> {
    let mut changed = Vec::new();
    let norm = |v: &Value| v.as_str().map(|s| s.trim().to_lowercase());
    if norm(&ai["merchant"]["value"]) != norm(&saved["merchant"]) {
        changed.push("merchant".to_string());
    }
    if ai["total"]["value"].as_i64() != saved["amount"].as_i64() {
        changed.push("total".to_string());
    }
    // Compare to the minute; seconds are rarely printed.
    let minute = |v: &Value| {
        v.as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .and_then(|d| d.with_timezone(&Utc).with_second(0))
    };
    if minute(&ai["date"]["value"]) != minute(&saved["occurred_at"]) {
        changed.push("date".to_string());
    }
    if ai["category_suggestion"]["value"].as_str() != chosen_slug {
        changed.push("category".to_string());
    }
    let ai_items: Vec<(Option<String>, Option<i64>)> = ai["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|i| (norm(&i["name"]), i["total"].as_i64()))
                .collect()
        })
        .unwrap_or_default();
    let saved_items: Vec<(Option<String>, Option<i64>)> = saved["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|i| (norm(&i["name"]), i["total"].as_i64()))
                .collect()
        })
        .unwrap_or_default();
    if ai_items != saved_items {
        changed.push("items".to_string());
    }
    changed
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<CreateTx>,
) -> AppResult<(StatusCode, Json<Transaction>)> {
    if req.kind != "expense" && req.kind != "income" {
        return Err(AppError::bad_request("kind must be expense or income"));
    }
    check_amount("amount", req.amount, false)?;
    let merchant = clean_text("merchant", &req.merchant, 120)?;
    let note = req
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string);
    if note.as_ref().is_some_and(|n| n.chars().count() > 500) {
        return Err(AppError::bad_request("note is too long (max 500)"));
    }
    for it in &req.items {
        check_amount("item total", it.total, true)?;
    }
    let category = check_category(&state, uid, req.category_id, &req.kind).await?;
    let occurred_at = req
        .occurred_at
        .unwrap_or_else(now_jakarta)
        .with_timezone(&Utc);
    let items = serde_json::to_value(&req.items).expect("serializable");

    let mut tx = state.db.begin().await?;
    let mut source = "manual";
    let mut correction: Option<(Value, Vec<String>)> = None;

    if let Some(rid) = req.receipt_id {
        let receipt = db::receipts::get_for_update(&mut tx, uid, rid)
            .await?
            .ok_or_else(|| AppError::bad_request("unknown receipt"))?;
        if receipt.status != "ready" && receipt.status != "needs_review" {
            return Err(AppError::Conflict(format!(
                "receipt is {}, it can't be saved as a transaction",
                receipt.status
            )));
        }
        source = "scan";
        let saved = json!({
            "merchant": merchant,
            "amount": req.amount,
            "occurred_at": occurred_at.with_timezone(&jakarta()).to_rfc3339(),
            "category": category.as_ref().map(|c| c.slug.clone()),
            "items": items,
        });
        let ai = receipt.extraction.unwrap_or(Value::Null);
        let changed = corrected_fields(&ai, &saved, category.as_ref().map(|c| c.slug.as_str()));
        correction = Some((saved, changed));
    }

    let created = db::transactions::insert(
        &mut tx,
        NewTransaction {
            user_id: uid,
            category_id: category.as_ref().map(|c| c.id),
            kind: &req.kind,
            amount: req.amount,
            merchant: &merchant,
            note: note.as_deref(),
            occurred_at,
            source,
            receipt_id: req.receipt_id,
            items,
            needs_check: req.needs_check,
        },
    )
    .await;
    let created = match created {
        Ok(t) => t,
        Err(e) if db::is_unique_violation(&e) => {
            return Err(AppError::Conflict("this receipt is already saved".into()));
        }
        Err(e) => return Err(e.into()),
    };

    if let (Some(rid), Some((saved, changed))) = (req.receipt_id, correction) {
        db::receipts::mark_confirmed(&mut tx, rid, &saved, &changed).await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn update(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTx>,
) -> AppResult<Json<Transaction>> {
    let existing = db::transactions::get(&state.db, uid, id)
        .await?
        .ok_or(AppError::NotFound)?;
    if let Some(a) = req.amount {
        check_amount("amount", a, false)?;
    }
    let merchant = req
        .merchant
        .as_deref()
        .map(|m| clean_text("merchant", m, 120))
        .transpose()?;
    if let Some(Some(cid)) = req.category_id {
        check_category(&state, uid, Some(cid), &existing.kind).await?;
    }
    let note = req
        .note
        .map(|n| n.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
    let updated = db::transactions::update(
        &state.db,
        uid,
        id,
        Patch {
            category_id: req.category_id,
            amount: req.amount,
            merchant: merchant.as_deref(),
            note: note.as_ref().map(|n| n.as_deref()),
            occurred_at: req.occurred_at.map(|d| d.with_timezone(&Utc)),
            needs_check: req.needs_check,
        },
    )
    .await?;
    updated.map(Json).ok_or(AppError::NotFound)
}

pub async fn remove(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if db::transactions::delete(&state.db, uid, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_detects_corrections() {
        let ai = json!({
            "merchant": { "value": "INDOMARET" },
            "total": { "value": 87400 },
            "date": { "value": "2026-10-15T19:05:00+07:00" },
            "category_suggestion": { "value": "groceries" },
            "items": [{ "name": "TELUR", "total": 27000 }]
        });
        let same = json!({
            "merchant": "Indomaret ",
            "amount": 87400,
            "occurred_at": "2026-10-15T19:05:30+07:00",
            "items": [{ "name": "telur", "total": 27000 }]
        });
        assert!(corrected_fields(&ai, &same, Some("groceries")).is_empty());

        let fixed = json!({
            "merchant": "Indomaret",
            "amount": 88400,
            "occurred_at": "2026-10-15T19:05:00+07:00",
            "items": [{ "name": "telur", "total": 28000 }]
        });
        assert_eq!(
            corrected_fields(&ai, &fixed, Some("food")),
            vec!["total", "category", "items"]
        );
    }
}
