use nvbes_core::config::AppConfig;
use nvbes_observability::{
    WorkerMonitorSchedule, capture_error_reporting_smoke, init_error_reporting, init_tracing,
    install_safe_panic_hook, start_continuous_profiling, start_worker_monitor_check_in,
    worker_monitor_slug,
};

#[path = "drive.workers.mod.rs"]
pub mod workers;

pub use nvbes_drive_api::db;

use db::Database;

const MAINTENANCE_ENQUEUE_MONITOR_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 1440,
    checkin_margin_minutes: 60,
    max_runtime_minutes: 30,
};
const DRIVE_WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_DRIVE_WORKER_METRICS_BIND_ADDR";
const WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_WORKER_METRICS_BIND_ADDR";
const DEFAULT_DRIVE_WORKER_METRICS_BIND_ADDR: &str = "127.0.0.1:4101";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    let _error_reporting_guard = init_error_reporting(&config);
    install_safe_panic_hook();
    init_tracing(&config);

    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("error-reporting-smoke")
    ) {
        let result =
            capture_error_reporting_smoke("drive-worker", &config.environment, "worker", false);
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let _profiling_guard =
        start_continuous_profiling(&config, "drive-worker").map_err(anyhow::Error::msg)?;

    let database = Database::connect(&config).await?;
    database.migrate().await?;
    let redis = nvbes_core::redis_runtime::require_redis_pool(&config).await?;
    let storage = nvbes_drive_api::app::build_storage(&config).await;

    let observability = nvbes_observability::metrics::HttpMetrics::default();
    let metrics_bind_addr = drive_worker_metrics_bind_addr();
    let _metrics_server = nvbes_observability::start_metrics_server(
        &config,
        observability.clone(),
        &metrics_bind_addr,
    )
    .await?;
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
                MAINTENANCE_ENQUEUE_MONITOR_SCHEDULE,
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
            workers::run_loop(&config, &database, &redis, storage, &observability).await?;
        }
        None | Some("run-once") => {
            let processed =
                workers::run_once(&config, &database, &redis, storage, &observability).await?;
            tracing::info!(processed, "worker run complete");
        }
        Some(other) => {
            anyhow::bail!("unsupported worker subcommand: {other}");
        }
    }

    Ok(())
}

fn drive_worker_metrics_bind_addr() -> String {
    std::env::var(DRIVE_WORKER_METRICS_BIND_ADDR_ENV)
        .or_else(|_| std::env::var(WORKER_METRICS_BIND_ADDR_ENV))
        .unwrap_or_else(|_| DEFAULT_DRIVE_WORKER_METRICS_BIND_ADDR.to_string())
}
