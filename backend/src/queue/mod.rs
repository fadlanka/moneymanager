//! Extraction worker. Receipts are queued in Postgres (status = 'pending'),
//! claimed with SKIP LOCKED, and processed one at a time through the rate limiter.

pub mod rate_limit;

use std::sync::Arc;
use std::time::{Duration, Instant};

use rate_limit::RateLimiter;
use tracing::Instrument;
use uuid::Uuid;

use crate::db::receipts::{self, Completion, Job};
use crate::extraction::prompt::PROMPT_VERSION;
use crate::extraction::{ExtractError, validate};
use crate::state::AppState;
use crate::time::now_jakarta;

/// Retry delay after a retryable failure: 30s, 60s, 120s, ...
fn retry_delay_secs(attempt: i32, err: &ExtractError) -> i64 {
    let base = if matches!(err, ExtractError::RateLimited(_)) {
        60
    } else {
        30
    };
    base * 2_i64.pow((attempt.max(1) - 1).min(6) as u32)
}

pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    let limiter = Arc::new(RateLimiter::per_minute(
        state.config.extraction.requests_per_minute,
    ));
    tokio::spawn(async move {
        match receipts::reset_stuck(&state.db).await {
            Ok(n) if n > 0 => tracing::warn!(count = n, "requeued receipts stuck in processing"),
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "reset_stuck failed"),
        }
        loop {
            match process_one(&state, &limiter).await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(e) => tracing::error!(error = %e, "worker database error"),
            }
            tokio::select! {
                _ = state.queue_signal.notified() => {}
                _ = tokio::time::sleep(state.config.extraction.poll_interval) => {}
            }
        }
    })
}

/// Claim and process one due receipt. Returns false when the queue is empty.
pub async fn process_one(state: &AppState, limiter: &RateLimiter) -> sqlx::Result<bool> {
    let Some(job) = receipts::claim_next(&state.db).await? else {
        return Ok(false);
    };
    let span = tracing::info_span!("extract", receipt = %job.id, attempt = job.attempts);
    process(state, limiter, job).instrument(span).await?;
    Ok(true)
}

async fn process(state: &AppState, limiter: &RateLimiter, job: Job) -> sqlx::Result<()> {
    let extractor = &state.extractor;

    let image = match state.storage.read(&job.image_path).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "image missing");
            return receipts::fail(
                &state.db,
                job.id,
                "image file missing",
                extractor.provider(),
                extractor.model(),
            )
            .await;
        }
    };

    limiter.acquire().await;
    let started = Instant::now();
    let result = extractor.extract(&image, &job.mime_type).await;
    let latency_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;

    match result {
        Ok(out) => {
            let v = validate::validate(
                out.extraction,
                now_jakarta(),
                state.config.extraction.review_threshold,
            );
            let status = if v.needs_review() {
                "needs_review"
            } else {
                "ready"
            };
            tracing::info!(
                status,
                latency_ms,
                issues = v.issues.len(),
                "receipt extracted"
            );
            receipts::complete(
                &state.db,
                job.id,
                Completion {
                    status,
                    provider: extractor.provider(),
                    model: extractor.model(),
                    prompt_version: PROMPT_VERSION,
                    extraction: serde_json::to_value(&v.extraction).expect("serializable"),
                    issues: serde_json::to_value(&v.issues).expect("serializable"),
                    latency_ms,
                    input_tokens: out.input_tokens,
                    output_tokens: out.output_tokens,
                },
            )
            .await
        }
        Err(err) => {
            if matches!(err, ExtractError::RateLimited(_)) {
                limiter.back_off(Duration::from_secs(60)).await;
            }
            if err.is_retryable() && job.attempts < state.config.extraction.max_attempts {
                let delay = retry_delay_secs(job.attempts, &err);
                tracing::warn!(error = %err, delay, "extraction failed, will retry");
                receipts::retry_later(&state.db, job.id, &err.to_string(), delay).await
            } else {
                tracing::error!(error = %err, "extraction failed permanently");
                receipts::fail(
                    &state.db,
                    job.id,
                    &err.to_string(),
                    extractor.provider(),
                    extractor.model(),
                )
                .await
            }
        }
    }
}

/// Convenience for handlers: wake the worker now instead of waiting for the poll.
pub fn notify(state: &AppState, receipt_id: Uuid) {
    tracing::debug!(%receipt_id, "receipt queued");
    state.queue_signal.notify_one();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows() {
        let t = ExtractError::Transient("x".into());
        let r = ExtractError::RateLimited("x".into());
        assert_eq!(retry_delay_secs(1, &t), 30);
        assert_eq!(retry_delay_secs(2, &t), 60);
        assert_eq!(retry_delay_secs(1, &r), 60);
        assert_eq!(retry_delay_secs(3, &r), 240);
    }
}
