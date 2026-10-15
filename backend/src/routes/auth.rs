use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::clean_text;
use crate::auth::password;
use crate::db::{self, users::User};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterReq {
    email: String,
    name: String,
    password: String,
    invite_code: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginReq {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResp {
    token: String,
    user: User,
}

fn normalize_email(raw: &str) -> AppResult<String> {
    let e = raw.trim().to_lowercase();
    let valid = e.len() <= 254
        && e.split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid {
        return Err(AppError::bad_request("enter a valid email"));
    }
    Ok(e)
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> AppResult<(StatusCode, Json<AuthResp>)> {
    // The app is for a small group of friends: registration needs the invite code.
    if let Some(expected) = &state.config.invite_code
        && req.invite_code.as_deref().map(str::trim) != Some(expected.as_str())
    {
        return Err(AppError::bad_request("invalid invite code"));
    }
    let email = normalize_email(&req.email)?;
    let name = clean_text("name", &req.name, 60)?;
    if req.password.chars().count() < 8 {
        return Err(AppError::bad_request(
            "password must be at least 8 characters",
        ));
    }
    let hash = password::hash(&req.password)?;

    let mut tx = state.db.begin().await?;
    let user = match db::users::create(&mut tx, &email, &name, &hash).await {
        Ok(u) => u,
        Err(e) if db::is_unique_violation(&e) => {
            return Err(AppError::Conflict(
                "this email is already registered".into(),
            ));
        }
        Err(e) => return Err(e.into()),
    };
    db::categories::seed_defaults(&mut tx, user.id).await?;
    tx.commit().await?;

    let token = state.jwt.issue(user.id).map_err(anyhow::Error::from)?;
    Ok((StatusCode::CREATED, Json(AuthResp { token, user })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> AppResult<Json<AuthResp>> {
    let email = req.email.trim().to_lowercase();
    let found = db::users::find_by_email(&state.db, &email).await?;
    // Same error for unknown email and wrong password.
    let (user, hash) = found.ok_or(AppError::Unauthorized)?;
    if !password::verify(&req.password, &hash) {
        return Err(AppError::Unauthorized);
    }
    let token = state.jwt.issue(user.id).map_err(anyhow::Error::from)?;
    Ok(Json(AuthResp { token, user }))
}
