use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub id: Uuid,
    pub status: String,
    #[serde(skip)]
    pub image_path: String,
    pub mime_type: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
    pub extraction: Option<Value>,
    pub issues: Value,
    pub error: Option<String>,
    pub attempts: i32,
    pub latency_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

/// A receipt claimed by the worker.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub user_id: Uuid,
    pub image_path: String,
    pub mime_type: String,
    pub attempts: i32,
}

pub struct Completion<'a> {
    pub status: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub prompt_version: &'a str,
    pub extraction: Value,
    pub issues: Value,
    pub latency_ms: i32,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

pub async fn create(
    db: &PgPool,
    id: Uuid,
    user_id: Uuid,
    image_path: &str,
    mime_type: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO receipts (id, user_id, image_path, mime_type) VALUES ($1, $2, $3, $4)",
        id,
        user_id,
        image_path,
        mime_type
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<Option<Receipt>> {
    sqlx::query_as!(
        Receipt,
        r#"SELECT id, status, image_path, mime_type, provider, model, prompt_version,
                  extraction, issues, error, attempts, latency_ms, created_at, processed_at
           FROM receipts WHERE user_id = $1 AND id = $2"#,
        user_id,
        id
    )
    .fetch_optional(db)
    .await
}

/// Same as `get` but locks the row, inside a transaction.
pub async fn get_for_update(
    conn: &mut PgConnection,
    user_id: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<Receipt>> {
    sqlx::query_as!(
        Receipt,
        r#"SELECT id, status, image_path, mime_type, provider, model, prompt_version,
                  extraction, issues, error, attempts, latency_ms, created_at, processed_at
           FROM receipts WHERE user_id = $1 AND id = $2 FOR UPDATE"#,
        user_id,
        id
    )
    .fetch_optional(conn)
    .await
}

/// Atomically take the oldest due receipt. SKIP LOCKED lets several workers run safely.
pub async fn claim_next(db: &PgPool) -> sqlx::Result<Option<Job>> {
    sqlx::query_as!(
        Job,
        r#"UPDATE receipts SET status = 'processing', attempts = attempts + 1, updated_at = now()
           WHERE id = (
               SELECT id FROM receipts
               WHERE status = 'pending' AND next_attempt_at <= now()
               ORDER BY next_attempt_at
               FOR UPDATE SKIP LOCKED
               LIMIT 1
           )
           RETURNING id, user_id, image_path, mime_type, attempts"#
    )
    .fetch_optional(db)
    .await
}

pub async fn complete(db: &PgPool, id: Uuid, c: Completion<'_>) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE receipts SET
             status = $2, provider = $3, model = $4, prompt_version = $5,
             extraction = $6, issues = $7, latency_ms = $8,
             input_tokens = $9, output_tokens = $10,
             error = NULL, processed_at = now(), updated_at = now()
           WHERE id = $1"#,
        id,
        c.status,
        c.provider,
        c.model,
        c.prompt_version,
        c.extraction,
        c.issues,
        c.latency_ms,
        c.input_tokens,
        c.output_tokens
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn retry_later(db: &PgPool, id: Uuid, error: &str, delay_secs: i64) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE receipts SET status = 'pending', error = $2,
             next_attempt_at = now() + make_interval(secs => $3), updated_at = now()
           WHERE id = $1"#,
        id,
        error,
        delay_secs as f64
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn fail(
    db: &PgPool,
    id: Uuid,
    error: &str,
    provider: &str,
    model: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE receipts SET status = 'failed', error = $2, provider = $3, model = $4,
             processed_at = now(), updated_at = now()
           WHERE id = $1"#,
        id,
        error,
        provider,
        model
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Requeue receipts left in `processing` by a crash or restart.
pub async fn reset_stuck(db: &PgPool) -> sqlx::Result<u64> {
    let res = sqlx::query!(
        r#"UPDATE receipts SET status = 'pending', updated_at = now()
           WHERE status = 'processing' AND updated_at < now() - interval '10 minutes'"#
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected())
}

/// Manual retry of a failed receipt. Returns false if it wasn't in `failed`.
pub async fn requeue_failed(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let res = sqlx::query!(
        r#"UPDATE receipts SET status = 'pending', attempts = 0, error = NULL,
             next_attempt_at = now(), updated_at = now()
           WHERE user_id = $1 AND id = $2 AND status = 'failed'"#,
        user_id,
        id
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn mark_confirmed(
    conn: &mut PgConnection,
    id: Uuid,
    confirmed: &Value,
    corrected_fields: &[String],
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE receipts SET status = 'confirmed', confirmed = $2, corrected_fields = $3, updated_at = now()
           WHERE id = $1"#,
        id,
        confirmed,
        corrected_fields
    )
    .execute(conn)
    .await?;
    Ok(())
}
