use nvbes_core::config::AppConfig;
use nvbes_observability::{
    WorkerMonitorSchedule, init_sentry, init_tracing, install_safe_panic_hook,
    start_worker_monitor_check_in, worker_monitor_slug,
};

#[path = "drive.workers.mod.rs"]
pub mod workers;

pub use nvbes_drive_api::db;

use db::Database;

const MAINTENANCE_ENQUEUE_SENTRY_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 1440,
    checkin_margin_minutes: 60,
    max_runtime_minutes: 30,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    let _sentry_guard = Box::leak(Box::new(init_sentry(&config)));
    install_safe_panic_hook();
    init_tracing(&config);

    let database = Database::connect(&config).await?;
    database.migrate().await?;
    let redis = nvbes_core::redis_runtime::require_redis_pool(&config).await?;
    let storage = nvbes_drive_api::app::build_storage(&config).await;

    let observability = nvbes_observability::metrics::HttpMetrics::default();
    observability.record_postgres_pool(
        &config.app_name,
        &config.environment,
        database.size(),
        database.num_idle(),
    );

    match std::env::args().nth(1).as_deref() {
        Some("enqueue-maintenance") => {
            let check_in = start_worker_monitor_check_in(
                &config.environment,
                &worker_monitor_slug("drive-worker", "enqueue-maintenance"),
                MAINTENANCE_ENQUEUE_SENTRY_SCHEDULE,
            );

            if let Err(error) = workers::enqueue_maintenance_jobs(&redis).await {
                check_in.finish_error();
                return Err(error);
            }

            check_in.finish_ok();
            tracing::info!("maintenance jobs enqueued");
        }
        Some("run-loop") => {
            tracing::info!("starting worker loop");
            workers::run_loop(&database, &redis, storage, &observability).await?;
        }
        None | Some("run-once") => {
            let processed = workers::run_once(&database, &redis, storage, &observability).await?;
            tracing::info!(processed, "worker run complete");
        }
        Some(other) => {
            anyhow::bail!("unsupported worker subcommand: {other}");
        }
    }

    Ok(())
}
