use nvbes_observability::metrics::HttpMetrics;
use nvbes_observability::worker_error_reporting::{
    WorkerOperationContext, capture_worker_operation_error,
};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info};

pub async fn run_outbox_tick(pool: &PgPool, environment: &str) {
    let metrics = HttpMetrics::new();
    metrics.record_worker_heartbeat("outbox");

    // 1. Dispatch pending Account outbox events
    let started_account = Instant::now();
    match nvbes_account_service::outbox::publish_pending(pool, 50).await {
        Ok(published) => {
            if published > 0 {
                info!(published, "published pending account outbox events");
                metrics.record_worker_queue_job("account_outbox", "ok", started_account.elapsed());
            }
        }
        Err(err) => {
            error!("error publishing account outbox events: {err}");
            let context = WorkerOperationContext {
                app_name: "nvbes-worker",
                environment,
                operation: "publish_account_outbox",
            };
            capture_worker_operation_error(&err, &context);
            metrics.record_worker_queue_job("account_outbox", "error", started_account.elapsed());
        }
    }

    // 2. Dispatch pending Billing outbox events
    let started_billing = Instant::now();
    match nvbes_billing_service::outbox::publish_pending_outbox_events(
        pool,
        None,
        "https://nvbes.test",
        50,
    )
    .await
    {
        Ok(published) => {
            if published > 0 {
                info!(published, "published pending billing outbox events");
                metrics.record_worker_queue_job("billing_outbox", "ok", started_billing.elapsed());
            }
        }
        Err(err) => {
            error!("error publishing billing outbox events: {err}");
            let context = WorkerOperationContext {
                app_name: "nvbes-worker",
                environment,
                operation: "publish_billing_outbox",
            };
            capture_worker_operation_error(&err, &context);
            metrics.record_worker_queue_job("billing_outbox", "error", started_billing.elapsed());
        }
    }
}
