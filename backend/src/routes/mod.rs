//! HTTP layer. Handlers stay thin: parse, check ownership, call db/domain, shape the response.

mod auth;
mod budgets;
mod categories;
mod debts;
mod health;
mod me;
mod receipts;
mod summary;
mod transactions;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::routing::{get, post, put};
use chrono::NaiveDate;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::time::{month_start, now_jakarta, parse_month};

/// Largest amount accepted anywhere (Rp 1 trillion). Guards against typos and overflow.
pub const MAX_AMOUNT: i64 = 1_000_000_000_000;

pub fn router(state: AppState) -> Router {
    // Multipart framing adds a little on top of the image itself.
    let upload_limit = state.config.max_upload_bytes + 64 * 1024;

    let api = Router::new()
        .route("/health", get(health::health))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/me", get(me::get_me).patch(me::update_me))
        .route(
            "/categories",
            get(categories::list).post(categories::create),
        )
        .route(
            "/receipts",
            post(receipts::upload).layer(DefaultBodyLimit::max(upload_limit)),
        )
        .route("/receipts/{id}", get(receipts::get_one))
        .route("/receipts/{id}/image", get(receipts::image))
        .route("/receipts/{id}/retry", post(receipts::retry))
        .route(
            "/transactions",
            get(transactions::list).post(transactions::create),
        )
        .route(
            "/transactions/{id}",
            get(transactions::get_one)
                .patch(transactions::update)
                .delete(transactions::remove),
        )
        .route("/summary", get(summary::summary))
        .route("/budgets", get(budgets::list).put(budgets::replace))
        .route("/budgets/apply-suggestion", post(budgets::apply_suggestion))
        .route("/debts", get(debts::list).post(debts::create))
        .route("/debts/plan", get(debts::plan))
        .route("/debts/{id}", put(debts::update).delete(debts::remove))
        .route("/debts/{id}/payments", post(debts::pay));

    let mut app = Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // CORS only matters for the web (PWA) build used on iPhones.
    let origins: Vec<HeaderValue> = state
        .config
        .cors_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    if !origins.is_empty() {
        app = app.layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        );
    }
    app
}

/// `?month=2026-10`, defaulting to the current month in Jakarta.
pub(crate) fn month_param(raw: Option<&str>) -> AppResult<NaiveDate> {
    match raw {
        None | Some("") => Ok(month_start(now_jakarta().date_naive())),
        Some(m) => {
            parse_month(m).ok_or_else(|| AppError::bad_request("month must look like 2026-10"))
        }
    }
}

pub(crate) fn check_amount(field: &str, v: i64, allow_zero: bool) -> AppResult<()> {
    let min = if allow_zero { 0 } else { 1 };
    if v < min || v > MAX_AMOUNT {
        return Err(AppError::bad_request(format!(
            "{field} must be between {min} and {MAX_AMOUNT} rupiah"
        )));
    }
    Ok(())
}

pub(crate) fn clean_text(field: &str, v: &str, max: usize) -> AppResult<String> {
    let t = v.trim();
    if t.is_empty() {
        return Err(AppError::bad_request(format!("{field} is required")));
    }
    if t.chars().count() > max {
        return Err(AppError::bad_request(format!(
            "{field} is too long (max {max})"
        )));
    }
    Ok(t.to_string())
}
