use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::queue;
use crate::state::AppState;
use crate::storage::sniff_image;

#[derive(Serialize)]
pub struct ReceiptView {
    id: Uuid,
    /// pending | processing | ready | needs_review | failed | confirmed
    status: String,
    extraction: Option<Value>,
    issues: Value,
    error: Option<String>,
    attempts: i32,
    provider: Option<String>,
    model: Option<String>,
    latency_ms: Option<i32>,
    /// The model's category suggestion resolved to this user's category id.
    suggested_category_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct Queued {
    id: Uuid,
    status: &'static str,
}

pub async fn upload(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Queued>)> {
    let mut image: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("invalid upload: {e}")))?
    {
        if field.name() == Some("image") {
            let bytes = field.bytes().await.map_err(|e| {
                if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    AppError::PayloadTooLarge
                } else {
                    AppError::bad_request(format!("invalid upload: {e}"))
                }
            })?;
            image = Some(bytes.to_vec());
            break;
        }
    }
    let bytes = image.ok_or_else(|| AppError::bad_request("missing form field `image`"))?;
    if bytes.len() > state.config.max_upload_bytes {
        return Err(AppError::PayloadTooLarge);
    }
    let (mime, ext) = sniff_image(&bytes).ok_or_else(|| {
        AppError::bad_request("only JPEG, PNG, WebP or HEIC photos are supported")
    })?;

    let id = Uuid::new_v4();
    let path = state
        .storage
        .save(uid, id, ext, &bytes)
        .await
        .map_err(|e| anyhow::anyhow!("saving image: {e}"))?;
    db::receipts::create(&state.db, id, uid, &path, mime).await?;
    queue::notify(&state, id);

    Ok((
        StatusCode::ACCEPTED,
        Json(Queued {
            id,
            status: "pending",
        }),
    ))
}

pub async fn get_one(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ReceiptView>> {
    let r = db::receipts::get(&state.db, uid, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let slug = r
        .extraction
        .as_ref()
        .and_then(|e| e["category_suggestion"]["value"].as_str())
        .map(str::to_string);
    let suggested_category_id = match slug {
        Some(s) => db::categories::find_by_slug(&state.db, uid, &s)
            .await?
            .map(|c| c.id),
        None => None,
    };
    Ok(Json(ReceiptView {
        id: r.id,
        status: r.status,
        extraction: r.extraction,
        issues: r.issues,
        error: r.error,
        attempts: r.attempts,
        provider: r.provider,
        model: r.model,
        latency_ms: r.latency_ms,
        suggested_category_id,
        created_at: r.created_at,
        processed_at: r.processed_at,
    }))
}

pub async fn image(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let r = db::receipts::get(&state.db, uid, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let bytes = state
        .storage
        .read(&r.image_path)
        .await
        .map_err(|_| AppError::NotFound)?;
    Response::builder()
        .header(header::CONTENT_TYPE, r.mime_type)
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(e.into()))
}

pub async fn retry(
    State(state): State<AppState>,
    AuthUser(uid): AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<(StatusCode, Json<Queued>)> {
    if db::receipts::get(&state.db, uid, id).await?.is_none() {
        return Err(AppError::NotFound);
    }
    if !db::receipts::requeue_failed(&state.db, uid, id).await? {
        return Err(AppError::Conflict(
            "only failed receipts can be retried".into(),
        ));
    }
    queue::notify(&state, id);
    Ok((
        StatusCode::ACCEPTED,
        Json(Queued {
            id,
            status: "pending",
        }),
    ))
}
