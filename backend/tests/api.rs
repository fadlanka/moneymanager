//! End-to-end API tests against a real Postgres (sqlx::test creates a fresh
//! database per test) with the mock extractor. Needs DATABASE_URL.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use budget_backend::config::{Config, ExtractionConfig, ProviderKind};
use budget_backend::extraction::mock::MockExtractor;
use budget_backend::queue::{self, rate_limit::RateLimiter};
use budget_backend::state::AppState;
use budget_backend::{build_state, routes};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const FIXTURE: &str = include_str!("../fixtures/indomaret.json");
const INVITE: &str = "teman-2026";
/// Smallest thing that sniffs as a JPEG; the mock extractor never decodes it.
const FAKE_JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1];

struct TestApp {
    app: Router,
    state: AppState,
    limiter: RateLimiter,
}

fn config() -> Config {
    Config {
        database_url: String::new(),
        bind_addr: String::new(),
        jwt_secret: "integration-test-secret-at-least-32-chars".into(),
        invite_code: Some(INVITE.into()),
        upload_dir: std::env::temp_dir().join(format!("budget-test-{}", uuid::Uuid::new_v4())),
        max_upload_bytes: 1024 * 1024,
        cors_origins: vec![],
        extraction: ExtractionConfig {
            provider: ProviderKind::Mock,
            gemini: None,
            openai: None,
            mock_fixture: None::<PathBuf>,
            requests_per_minute: 6000,
            timeout: Duration::from_secs(5),
            max_attempts: 3,
            poll_interval: Duration::from_secs(1),
            review_threshold: 0.75,
        },
    }
}

fn setup(db: PgPool) -> TestApp {
    let extractor = Arc::new(MockExtractor::new(FIXTURE, Duration::ZERO));
    let state = build_state(config(), db, extractor);
    TestApp {
        app: routes::router(state.clone()),
        state,
        limiter: RateLimiter::per_minute(6000),
    }
}

impl TestApp {
    async fn call(&self, method: Method, path: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(format!("/api{path}"));
        if let Some(t) = token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => req.body(Body::empty()).unwrap(),
        };
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }

    async fn upload(&self, token: &str, bytes: &[u8]) -> (StatusCode, Value) {
        let boundary = "XBOUNDARYX";
        let mut body = Vec::new();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"r.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/receipts")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
            .body(Body::from(body))
            .unwrap();
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn register(&self, email: &str) -> String {
        let (s, v) = self
            .call(
                Method::POST,
                "/auth/register",
                None,
                Some(json!({ "email": email, "name": "Tester", "password": "rahasia123", "invite_code": INVITE })),
            )
            .await;
        assert_eq!(s, StatusCode::CREATED, "{v}");
        v["token"].as_str().unwrap().to_string()
    }

    async fn drain_queue(&self) {
        while queue::process_one(&self.state, &self.limiter).await.unwrap() {}
    }

    async fn category_id(&self, token: &str, slug: &str) -> String {
        let (_, cats) = self.call(Method::GET, "/categories", Some(token), None).await;
        cats.as_array()
            .unwrap()
            .iter()
            .find(|c| c["slug"] == slug)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

#[sqlx::test(migrator = "budget_backend::MIGRATOR")]
async fn auth_rules(db: PgPool) {
    let t = setup(db);
    let (s, _) = t
        .call(
            Method::POST,
            "/auth/register",
            None,
            Some(json!({ "email": "a@b.co", "name": "A", "password": "rahasia123", "invite_code": "wrong" })),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "invite code is required");

    let token = t.register("Fadlan@Example.com").await;
    let (s, me) = t.call(Method::GET, "/me", Some(&token), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(me["email"], "fadlan@example.com");

    let (s, _) = t
        .call(
            Method::POST,
            "/auth/register",
            None,
            Some(json!({ "email": "fadlan@example.com", "name": "B", "password": "rahasia123", "invite_code": INVITE })),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    let (s, _) = t
        .call(Method::POST, "/auth/login", None, Some(json!({ "email": "fadlan@example.com", "password": "salah123" })))
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, v) = t
        .call(Method::POST, "/auth/login", None, Some(json!({ "email": "FADLAN@example.com", "password": "rahasia123" })))
        .await;
    assert_eq!(s, StatusCode::OK);
    assert!(v["token"].is_string());

    let (s, _) = t.call(Method::GET, "/summary", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = t.call(Method::GET, "/summary", Some("garbage"), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (_, cats) = t.call(Method::GET, "/categories", Some(&token), None).await;
    assert!(cats.as_array().unwrap().len() >= 10, "default categories are seeded");
}

#[sqlx::test(migrator = "budget_backend::MIGRATOR")]
async fn receipt_scan_to_transaction(db: PgPool) {
    let t = setup(db.clone());
    let token = t.register("fadlan@example.com").await;

    let (s, v) = t.upload(&token, b"%PDF-1.4 not an image").await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{v}");

    let (s, queued) = t.upload(&token, FAKE_JPEG).await;
    assert_eq!(s, StatusCode::ACCEPTED, "{queued}");
    let rid = queued["id"].as_str().unwrap().to_string();

    let (_, r) = t.call(Method::GET, &format!("/receipts/{rid}"), Some(&token), None).await;
    assert_eq!(r["status"], "pending");

    t.drain_queue().await;

    let (s, r) = t.call(Method::GET, &format!("/receipts/{rid}"), Some(&token), None).await;
    assert_eq!(s, StatusCode::OK);
    // The fixture has two low-confidence lines, so the app must highlight them.
    assert_eq!(r["status"], "needs_review", "{r}");
    assert_eq!(r["extraction"]["total"]["value"], 87400);
    assert_eq!(r["extraction"]["items"][3]["flagged"], true);
    assert!(r["issues"].as_array().unwrap().iter().any(|i| i["field"] == "items[3]"));
    let groceries = t.category_id(&token, "groceries").await;
    assert_eq!(r["suggested_category_id"], groceries.as_str());

    // The user fixes the eggs line (27.000 → 28.000) and the total, keeps the rest.
    let (s, tx) = t
        .call(
            Method::POST,
            "/transactions",
            Some(&token),
            Some(json!({
                "amount": 88400,
                "merchant": "Indomaret",
                "occurred_at": "2026-10-15T19:05:00+07:00",
                "category_id": groceries,
                "receipt_id": rid,
                "items": [
                    { "name": "INDOMIE GRG", "qty": 5, "unit_price": 3500, "total": 17500 },
                    { "name": "UHT MILK 1L", "qty": 1, "unit_price": 19900, "total": 19900 },
                    { "name": "ROTI TAWAR", "qty": 1, "unit_price": 16500, "total": 16500 },
                    { "name": "TELUR 10S", "qty": 1, "unit_price": 28000, "total": 28000 },
                    { "name": "SABUN CUCI PIRING", "qty": 1, "unit_price": 6500, "total": 6500 }
                ]
            })),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{tx}");
    assert_eq!(tx["source"], "scan");

    let corrected: Vec<String> = sqlx::query_scalar("SELECT unnest(corrected_fields) FROM receipts WHERE id = $1::uuid")
        .bind(&rid)
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(corrected, vec!["total", "items"], "the correction diff is stored for evaluation");

    let (_, r) = t.call(Method::GET, &format!("/receipts/{rid}"), Some(&token), None).await;
    assert_eq!(r["status"], "confirmed");

    // Saving the same receipt twice is refused.
    let (s, _) = t
        .call(
            Method::POST,
            "/transactions",
            Some(&token),
            Some(json!({ "amount": 1000, "merchant": "x", "receipt_id": rid })),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    // Another user can't see any of it.
    let other = t.register("teman@example.com").await;
    let (s, _) = t.call(Method::GET, &format!("/receipts/{rid}"), Some(&other), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = t.call(Method::GET, &format!("/receipts/{rid}/image"), Some(&other), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let txid = tx["id"].as_str().unwrap();
    let (s, _) = t.call(Method::GET, &format!("/transactions/{txid}"), Some(&other), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (_, list) = t.call(Method::GET, "/transactions", Some(&other), None).await;
    assert_eq!(list.as_array().unwrap().len(), 0);
    // ...and can't attach someone else's category either.
    let (s, _) = t
        .call(
            Method::POST,
            "/transactions",
            Some(&other),
            Some(json!({ "amount": 1000, "merchant": "x", "category_id": groceries })),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrator = "budget_backend::MIGRATOR")]
async fn budgets_debts_and_summary(db: PgPool) {
    let t = setup(db);
    let token = t.register("fadlan@example.com").await;
    let food = t.category_id(&token, "food").await;
    let shopping = t.category_id(&token, "shopping").await;
    let housing = t.category_id(&token, "housing").await;

    let (s, _) = t
        .call(Method::PATCH, "/me", Some(&token), Some(json!({ "monthly_income": 3_300_000 })))
        .await;
    assert_eq!(s, StatusCode::OK);

    let (s, status) = t
        .call(
            Method::PUT,
            "/budgets",
            Some(&token),
            Some(json!({ "month": current_month(), "items": [
                { "category_id": food, "limit": 1_000_000 },
                { "category_id": shopping, "limit": 500_000 },
                { "category_id": housing, "limit": 1_200_000 }
            ] })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{status}");
    assert_eq!(status["total_limit"], 2_700_000);
    assert!(status["suggestion"].is_null());

    // A 750k/month installment pushes commitments over income.
    let (s, debt) = t
        .call(
            Method::POST,
            "/debts",
            Some(&token),
            Some(json!({ "name": "Cicilan laptop", "principal": 3_000_000, "monthly_payment": 750_000, "due_day": 28 })),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{debt}");
    let debt_id = debt["id"].as_str().unwrap().to_string();

    let (s, sum) = t.call(Method::GET, "/summary", Some(&token), None).await;
    assert_eq!(s, StatusCode::OK);
    let sug = &sum["budget"]["suggestion"];
    assert_eq!(sug["shortfall"], 150_000, "{sum}");
    assert!(sug["cuts"].as_array().unwrap().iter().all(|c| c["name"] != "Housing"));

    let (s, after) = t
        .call(Method::POST, "/budgets/apply-suggestion", Some(&token), Some(json!({})))
        .await;
    assert_eq!(s, StatusCode::OK, "{after}");
    assert_eq!(after["total_limit"], 2_550_000);
    assert!(after["suggestion"].is_null());
    let (s, _) = t
        .call(Method::POST, "/budgets/apply-suggestion", Some(&token), Some(json!({})))
        .await;
    assert_eq!(s, StatusCode::CONFLICT, "nothing left to apply");

    // Paying the installment records an expense and lowers the balance.
    let (s, paid) = t
        .call(Method::POST, &format!("/debts/{debt_id}/payments"), Some(&token), Some(json!({ "amount": 750_000 })))
        .await;
    assert_eq!(s, StatusCode::CREATED, "{paid}");
    assert_eq!(paid["debt"]["balance"], 2_250_000);
    assert_eq!(paid["transaction"]["source"], "debt_payment");
    let (s, _) = t
        .call(Method::POST, &format!("/debts/{debt_id}/payments"), Some(&token), Some(json!({ "amount": 9_000_000 })))
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "overpaying is refused");

    let (_, sum) = t.call(Method::GET, "/summary", Some(&token), None).await;
    assert_eq!(sum["expense"], 750_000);
    assert_eq!(sum["budget"]["upcoming_total"], 0, "this month's installment is paid");

    let (s, plan) = t.call(Method::GET, "/debts/plan?strategy=snowball&extra=250000", Some(&token), None).await;
    assert_eq!(s, StatusCode::OK, "{plan}");
    assert_eq!(plan["months"], 3);
    assert_eq!(plan["feasible"], true);

    // Manual income + expense and the month totals.
    let income = t.category_id(&token, "income").await;
    let (s, _) = t
        .call(
            Method::POST,
            "/transactions",
            Some(&token),
            Some(json!({ "kind": "income", "amount": 1_500_000, "merchant": "Freelance invoice", "category_id": income })),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = t
        .call(
            Method::POST,
            "/transactions",
            Some(&token),
            Some(json!({ "kind": "income", "amount": 10, "merchant": "x", "category_id": food })),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "income can't use an expense category");
    let (_, sum) = t.call(Method::GET, "/summary", Some(&token), None).await;
    assert_eq!(sum["income"], 1_500_000);
    assert_eq!(sum["net"], 750_000);

    let (_, found) = t.call(Method::GET, "/transactions?q=laptop", Some(&token), None).await;
    assert_eq!(found.as_array().unwrap().len(), 1);
}

fn current_month() -> String {
    budget_backend::time::now_jakarta().format("%Y-%m").to_string()
}
