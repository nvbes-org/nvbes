use std::error::Error;

use sentry::protocol::{Level, Value};
use uuid::Uuid;

use crate::error_reporting::is_error_reporting_configured;

#[derive(Debug)]
pub struct WorkerJobContext<'a> {
    pub app_name: &'a str,
    pub environment: &'a str,
    pub queue: &'a str,
    pub job_type: &'a str,
    pub job_id: Uuid,
    pub attempts: u32,
    pub max_attempts: u32,
}

#[derive(Debug)]
pub struct WorkerOperationContext<'a> {
    pub app_name: &'a str,
    pub environment: &'a str,
    pub operation: &'a str,
}

pub fn capture_worker_job_error(
    error: &(dyn Error + Send + Sync + 'static),
    context: &WorkerJobContext<'_>,
) {
    if is_error_reporting_configured() {
        sentry::with_scope(
            |scope| {
                scope.set_level(Some(Level::Error));
                scope.set_tag("app.name", context.app_name);
                scope.set_tag("environment", context.environment);
                scope.set_tag("runtime", "worker");
                scope.set_tag("queue", context.queue);
                scope.set_tag("job_type", context.job_type);
                scope.set_extra("job_id", Value::from(context.job_id.to_string()));
                scope.set_extra("attempts", Value::from(context.attempts));
                scope.set_extra("max_attempts", Value::from(context.max_attempts));
                scope.set_fingerprint(Some(&[
                    "worker_job",
                    context.app_name,
                    context.queue,
                    context.job_type,
                ]));
            },
            || sentry::capture_error(error),
        );
    }
    tracing::debug!(
        app = context.app_name,
        environment = context.environment,
        queue = context.queue,
        job_type = context.job_type,
        job_id = %context.job_id,
        attempts = context.attempts,
        max_attempts = context.max_attempts,
        "worker job error observed"
    );
}

pub fn capture_worker_operation_error(
    error: &(dyn Error + Send + Sync + 'static),
    context: &WorkerOperationContext<'_>,
) {
    if !is_error_reporting_configured() {
        return;
    }

    sentry::with_scope(
        |scope| {
            scope.set_level(Some(Level::Error));
            scope.set_tag("app.name", context.app_name);
            scope.set_tag("environment", context.environment);
            scope.set_tag("runtime", "worker");
            scope.set_tag("operation", context.operation);
            scope.set_fingerprint(Some(&[
                "worker_operation",
                context.app_name,
                context.operation,
            ]));
        },
        || sentry::capture_error(error),
    );
}
