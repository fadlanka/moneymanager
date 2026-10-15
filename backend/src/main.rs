//! Wiring only: config, database, extractor, worker, HTTP server.

use anyhow::Context;
use budget_backend::{MIGRATOR, build_state, config::Config, extraction, queue, routes};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info,sqlx=warn".into()),
        )
        .init();

    let config = Config::from_env()?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connecting to Postgres")?;
    MIGRATOR.run(&db).await.context("running migrations")?;

    let extractor = extraction::build_extractor(&config.extraction)?;
    let bind_addr = config.bind_addr.clone();
    let state = build_state(config, db, extractor);
    state
        .storage
        .ensure_root()
        .await
        .context("creating upload dir")?;

    let worker = queue::spawn(state.clone());
    let app = routes::router(state.clone());

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!(
        addr = %bind_addr,
        provider = state.extractor.provider(),
        model = state.extractor.model(),
        "listening"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    worker.abort();
    Ok(())
}

/// Ctrl-C locally, SIGTERM from `docker compose down`.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
