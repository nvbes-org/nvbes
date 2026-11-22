use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::info;

use crate::{config::ApiConfig, routes::build_router};

pub async fn run_server(config: ApiConfig) -> anyhow::Result<()> {
    info!(
        "initializing serverless database connection pool (min: {}, max: {})",
        config.database_min_connections, config.database_max_connections
    );

    let pool = PgPoolOptions::new()
        .min_connections(config.database_min_connections)
        .max_connections(config.database_max_connections)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(10))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO identity, account, billing, consent, public") // nvbes-allow-runtime-sql
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect_lazy(&config.database_url)
        .context("failed to configure lazy Postgres connection pool")?;

    let app = build_router(pool.clone(), &config);

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind to address {}", config.bind_addr))?;

    info!(
        "nvbes-api serverless monolith listening on http://{}",
        config.bind_addr
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error during execution")?;

    info!("server shutdown complete, closing database connections");
    pool.close().await;

    nvbes_observability::shutdown_tracing();
    let _ = nvbes_observability::flush_error_reporting(Duration::from_secs(1));

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install ctrl+c signal handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!("failed to install SIGTERM signal handler: {err}");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("received CTRL+C signal, starting graceful shutdown");
        }
        _ = terminate => {
            info!("received SIGTERM signal, starting graceful serverless shutdown");
        }
    }
}
