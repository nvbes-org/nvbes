use nvbes_observability::metrics::HttpMetrics;
use nvbes_observability::worker_error_reporting::{
    WorkerOperationContext, capture_worker_operation_error,
};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info};

pub async fn run_privacy_tick(pool: &PgPool, environment: &str) {
    let metrics = HttpMetrics::new();
    metrics.record_worker_heartbeat("privacy");
    let started = Instant::now();

    match nvbes_account_service::privacy_jobs::process_pending(pool).await {
        Ok(result) => {
            if result.exports_completed > 0 || result.closures_completed > 0 {
                info!(
                    exports_completed = result.exports_completed,
                    closures_completed = result.closures_completed,
                    "processed pending privacy retention jobs"
                );
                metrics.record_worker_queue_job("privacy_retention", "ok", started.elapsed());
            }
        }
        Err(err) => {
            error!("error processing privacy retention jobs: {err}");
            let context = WorkerOperationContext {
                app_name: "nvbes-worker",
                environment,
                operation: "privacy_retention_jobs",
            };
            capture_worker_operation_error(&err, &context);
            metrics.record_worker_queue_job("privacy_retention", "error", started.elapsed());
        }
    }
}
