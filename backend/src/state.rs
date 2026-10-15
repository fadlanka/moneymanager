use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Notify;

use crate::auth::jwt::JwtKeys;
use crate::config::Config;
use crate::extraction::ReceiptExtractor;
use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub storage: Storage,
    pub jwt: Arc<JwtKeys>,
    pub extractor: Arc<dyn ReceiptExtractor>,
    /// Wakes the extraction worker as soon as a receipt is uploaded.
    pub queue_signal: Arc<Notify>,
}
