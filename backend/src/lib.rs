pub mod auth;
pub mod budget;
pub mod config;
pub mod db;
pub mod error;
pub mod extraction;
pub mod queue;
pub mod routes;
pub mod state;
pub mod storage;
pub mod time;

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Notify;

use auth::jwt::JwtKeys;
use config::Config;
use extraction::ReceiptExtractor;
use state::AppState;
use storage::Storage;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Assemble shared state. Used by `main` and by integration tests.
pub fn build_state(config: Config, db: PgPool, extractor: Arc<dyn ReceiptExtractor>) -> AppState {
    AppState {
        storage: Storage::new(config.upload_dir.clone()),
        jwt: Arc::new(JwtKeys::new(&config.jwt_secret)),
        config: Arc::new(config),
        db,
        extractor,
        queue_signal: Arc::new(Notify::new()),
    }
}
