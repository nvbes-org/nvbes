use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::info;

use crate::{config::WorkerConfig, health, outbox, privacy};

pub async fn run_worker(config: WorkerConfig) -> anyhow::Result<()> {
    info!(
        "initializing serverless database connection pool (min: {}, max: {})",
        config.database_min_connections, config.database_max_connections
    );

    let pool = PgPoolOptions::new()
        .min_connections(config.database_min_connections)
        .max_connections(config.database_max_connections)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(10))
        .connect_lazy(&config.database_url)
        .context("failed to configure lazy Postgres connection pool")?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn HTTP health server for liveness and readiness probes
    let health_router = health::router().with_state(pool.clone());
    let listener = TcpListener::bind(config.bind_addr).await.with_context(|| {
        format!(
            "failed to bind worker health server to {}",
            config.bind_addr
        )
    })?;

    info!(
        "nvbes-worker health server listening on http://{}",
        config.bind_addr
    );

    let health_server = {
        let mut shutdown_rx = shutdown_rx.clone();
        async move {
            let _ = axum::serve(listener, health_router)
                .with_graceful_shutdown(async move {
                    while !*shutdown_rx.borrow_and_update() {
                        if shutdown_rx.changed().await.is_err() {
                            break;
                        }
                    }
                })
                .await;
        }
    };

    let worker_loop = run_worker_loop(pool.clone(), config, shutdown_rx);

    tokio::select! {
        _ = health_server => {
            info!("health server stopped");
        }
        _ = worker_loop => {
            info!("worker loop stopped");
        }
        _ = shutdown_signal() => {
            info!("shutdown signal received, stopping worker");
            let _ = shutdown_tx.send(true);
        }
    }

    info!("worker shutdown complete, closing database connections");
    pool.close().await;

    Ok(())
}

async fn run_worker_loop(
    pool: sqlx::PgPool,
    config: WorkerConfig,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut outbox_interval = tokio::time::interval(Duration::from_secs(config.poll_interval_secs));
    let mut privacy_interval =
        tokio::time::interval(Duration::from_secs(config.privacy_interval_secs));

    loop {
        tokio::select! {
            _ = outbox_interval.tick() => {
                outbox::run_outbox_tick(&pool).await;
            }
            _ = privacy_interval.tick() => {
                privacy::run_privacy_tick(&pool).await;
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
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
            info!("received CTRL+C signal");
        }
        _ = terminate => {
            info!("received SIGTERM signal");
        }
    }
}
