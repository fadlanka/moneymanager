use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::error::AppResult;
use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> AppResult<Json<Value>> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(json!({
        "ok": true,
        "extractor": { "provider": state.extractor.provider(), "model": state.extractor.model() }
    })))
}
