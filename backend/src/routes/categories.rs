use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;

use super::clean_text;
use crate::auth::AuthUser;
use crate::db::{self, categories::Category};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const COLORS: &[&str] = &[
    "yellow",
    "red",
    "green",
    "blue",
    "ink",
    "grey",
    "yellow-soft",
    "red-soft",
    "green-soft",
    "blue-soft",
];

#[derive(Deserialize)]
pub struct CreateCategory {
    name: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default = "default_color")]
    color: String,
    #[serde(default = "default_true")]
    flexible: bool,
}

fn default_kind() -> String {
    "expense".into()
}
fn default_color() -> String {
    "grey".into()
}
fn default_true() -> bool {
    true
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
) -> AppResult<Json<Vec<Category>>> {
    Ok(Json(db::categories::list(&state.db, uid).await?))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Json(req): Json<CreateCategory>,
) -> AppResult<(StatusCode, Json<Category>)> {
    let name = clean_text("name", &req.name, 40)?;
    if !["expense", "income"].contains(&req.kind.as_str()) {
        return Err(AppError::bad_request("kind must be expense or income"));
    }
    if !COLORS.contains(&req.color.as_str()) {
        return Err(AppError::bad_request(format!(
            "color must be one of {}",
            COLORS.join(", ")
        )));
    }
    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(AppError::bad_request(
            "name needs at least one letter or digit",
        ));
    }
    match db::categories::create(
        &state.db,
        uid,
        &slug,
        &name,
        &req.kind,
        &req.color,
        req.flexible,
    )
    .await
    {
        Ok(c) => Ok((StatusCode::CREATED, Json(c))),
        Err(e) if db::is_unique_violation(&e) => Err(AppError::Conflict(
            "a category with this name exists".into(),
        )),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugs() {
        assert_eq!(slugify("Food & drinks"), "food-drinks");
        assert_eq!(slugify("  Kopi!! "), "kopi");
        assert_eq!(slugify("🙂"), "");
    }
}
