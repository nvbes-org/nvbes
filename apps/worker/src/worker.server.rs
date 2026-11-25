use std::{sync::Arc, time::Duration, time::Instant};

use anyhow::Context;
use nvbes_observability::{
    metrics::HttpMetrics,
    worker_error_reporting::{WorkerOperationContext, capture_worker_operation_error},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, sync::watch, task::JoinSet};
use tracing::{error, info};

use crate::{
    config::{ServeConfig, WorkerConfig, env_lookup},
    health,
    tasks::{PreparedTask, WorkerTask},
};

const APP_NAME: &str = "nvbes-worker";

/// Domain tables live in their own schemas; every connection resolves them
/// through the same search path as the API.
pub fn pool(config: &WorkerConfig) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET search_path TO identity, account, public") // nvbes-allow-runtime-sql
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .min_connections(config.database_min_connections)
        .max_connections(config.database_max_connections)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(10))
        .connect_lazy(&config.database_url)
        .context("failed to configure lazy Postgres connection pool")
}

/// Scaleway Job mode: one batch, a JSON summary on stdout, a non-zero exit on
/// failure so the job is reported as failed.
pub async fn run_once(config: WorkerConfig, task: WorkerTask) -> anyhow::Result<()> {
    let prepared = PreparedTask::prepare(task, &env_lookup, config.development())
        .with_context(|| format!("invalid configuration for {task}"))?;
    let pool = pool(&config)?;
    let result = execute(&prepared, &pool, &config.environment).await;
    pool.close().await;
    let summary = result?;
    println!(
        "{}",
        serde_json::json!({"task": task.name(), "result": summary})
    );
    Ok(())
}

/// Scaleway Container mode: health endpoints plus one loop per scheduled task.
pub async fn serve(config: WorkerConfig) -> anyhow::Result<()> {
    let serve = ServeConfig::from_lookup(&env_lookup, config.development())
        .context("invalid serve configuration")?;
    let mut tasks = Vec::with_capacity(serve.schedules.len());
    for schedule in &serve.schedules {
        let prepared = PreparedTask::prepare(schedule.task, &env_lookup, config.development())
            .with_context(|| format!("invalid configuration for {}", schedule.task))?;
        tasks.push((Arc::new(prepared), schedule.every));
    }
    let pool = pool(&config)?;
    let listener = TcpListener::bind(serve.bind_addr)
        .await
        .with_context(|| format!("failed to bind worker health server to {}", serve.bind_addr))?;
    info!(
        "{APP_NAME} health server listening on http://{}",
        serve.bind_addr
    );

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut loops = JoinSet::new();
    for (task, every) in tasks {
        let pool = pool.clone();
        let environment = config.environment.clone();
        let mut shutdown = shutdown_rx.clone();
        loops.spawn(async move {
            let mut interval = tokio::time::interval(every);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Errors are reported and the next tick retries.
                        let _ = execute(&task, &pool, &environment).await;
                    }
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        });
    }

    let health_router = health::router(pool.clone(), &config);
    let mut health_shutdown = shutdown_rx.clone();
    let health_server = axum::serve(listener, health_router).with_graceful_shutdown(async move {
        while !*health_shutdown.borrow_and_update() {
            if health_shutdown.changed().await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        result = health_server => {
            result.context("worker health server failed")?;
        }
        _ = shutdown_signal() => {
            info!("shutdown signal received, stopping worker");
        }
    }
    let _ = shutdown_tx.send(true);
    while loops.join_next().await.is_some() {}
    info!("worker shutdown complete, closing database connections");
    pool.close().await;
    Ok(())
}

async fn execute(
    task: &PreparedTask,
    pool: &PgPool,
    environment: &str,
) -> anyhow::Result<serde_json::Value> {
    let metrics = HttpMetrics::new();
    let name = task.task().name();
    metrics.record_worker_heartbeat(name);
    let started = Instant::now();
    match task.execute(pool).await {
        Ok(summary) => {
            info!(task = name, %summary, "worker task completed");
            metrics.record_worker_queue_job(name, "ok", started.elapsed());
            Ok(summary)
        }
        Err(err) => {
            error!(task = name, "worker task failed: {err:#}");
            capture_worker_operation_error(
                &*err,
                &WorkerOperationContext {
                    app_name: APP_NAME,
                    environment,
                    operation: name,
                },
            );
            metrics.record_worker_queue_job(name, "error", started.elapsed());
            Err(err)
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!("failed to install ctrl+c signal handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                error!("failed to install SIGTERM signal handler: {err}");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received CTRL+C signal"),
        _ = terminate => info!("received SIGTERM signal"),
    }
}
