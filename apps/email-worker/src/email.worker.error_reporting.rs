use std::error::Error;

use nvbes_observability::{
    ErrorReportingConfig, ErrorReportingGuard, WorkerJobContext, WorkerMonitorSchedule,
    WorkerOperationContext, capture_error_reporting_smoke, capture_worker_heartbeat,
    capture_worker_job_error, capture_worker_operation_error, init_error_reporting_with_config,
    worker_monitor_slug,
};
use uuid::Uuid;

use crate::config::EmailWorkerConfig;

pub const APP_NAME: &str = "email-worker";
const DISPATCHER_MONITOR_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 5,
    checkin_margin_minutes: 2,
    max_runtime_minutes: 1,
};

pub fn init(config: &EmailWorkerConfig) -> ErrorReportingGuard {
    init_error_reporting_with_config(ErrorReportingConfig {
        app_name: APP_NAME,
        service_name: APP_NAME,
        environment: &config.environment,
        dsn: config.sentry_dsn.as_deref(),
        traces_sample_rate: config.sentry_traces_sample_rate,
    })
}

pub fn smoke(config: &EmailWorkerConfig) -> nvbes_observability::ErrorReportingSmokeResult {
    capture_error_reporting_smoke(
        APP_NAME,
        &config.environment,
        "worker",
        config.sentry_dsn.is_some(),
    )
}

pub fn capture_operation(
    config: &EmailWorkerConfig,
    operation: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) {
    capture_worker_operation_error(
        error,
        &WorkerOperationContext {
            app_name: APP_NAME,
            environment: &config.environment,
            operation,
        },
    );
}

pub fn capture_operation_message(
    config: &EmailWorkerConfig,
    operation: &'static str,
    message: &'static str,
) {
    capture_operation(config, operation, &std::io::Error::other(message));
}

pub fn capture_dispatcher_heartbeat(config: &EmailWorkerConfig) {
    capture_worker_heartbeat(
        &config.environment,
        &worker_monitor_slug(APP_NAME, "dispatcher-heartbeat"),
        DISPATCHER_MONITOR_SCHEDULE,
    );
}

pub fn capture_delivery(
    config: &EmailWorkerConfig,
    job_id: Uuid,
    job_type: &str,
    attempts: u32,
    max_attempts: u32,
    error: &(dyn Error + Send + Sync + 'static),
) {
    capture_worker_job_error(
        error,
        &WorkerJobContext {
            app_name: APP_NAME,
            environment: &config.environment,
            queue: "email_delivery",
            job_type,
            job_id,
            attempts,
            max_attempts,
        },
    );
}
