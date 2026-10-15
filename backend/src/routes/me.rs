use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use super::{check_amount, clean_text};
use crate::auth::AuthUser;
use crate::db::{self, users::User};
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct UpdateMe {
    name: Option<String>,
    /// Planned monthly income; the budget planner compares limits against it.
    monthly_income: Option<i64>,
}

pub async fn get_me(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
) -> AppResult<Json<User>> {
    Ok(Json(db::users::get(&state.db, uid).await?))
}

pub async fn update_me(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<UpdateMe>,
) -> AppResult<Json<User>> {
    let name = req
        .name
        .as_deref()
        .map(|n| clean_text("name", n, 60))
        .transpose()?;
    if let Some(income) = req.monthly_income {
        check_amount("monthly_income", income, true)?;
    }
    Ok(Json(
        db::users::update(&state.db, uid, name.as_deref(), req.monthly_income).await?,
    ))
}
