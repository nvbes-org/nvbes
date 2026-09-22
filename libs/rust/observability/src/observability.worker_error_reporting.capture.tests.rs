use std::io;

use uuid::Uuid;

use crate::error_reporting::{
    ErrorReportingConfig, init_error_reporting_with_config, is_error_reporting_configured,
};

use super::{
    WorkerJobContext, WorkerOperationContext, capture_worker_job_error,
    capture_worker_operation_error,
};

#[test]
fn capture_worker_job_error_is_noop_when_unconfigured() {
    let error = io::Error::other("job failed");
    capture_worker_job_error(
        &error,
        &WorkerJobContext {
            app_name: "email-worker",
            environment: "test",
            queue: "email",
            job_type: "send",
            job_id: Uuid::nil(),
            attempts: 1,
            max_attempts: 3,
        },
    );
}

#[test]
fn capture_worker_operation_error_returns_early_when_unconfigured() {
    let error = io::Error::other("operation failed");
    capture_worker_operation_error(
        &error,
        &WorkerOperationContext {
            app_name: "email-worker",
            environment: "test",
            operation: "heartbeat",
        },
    );
}

#[test]
fn worker_contexts_expose_expected_fields() {
    let job = WorkerJobContext {
        app_name: "app",
        environment: "env",
        queue: "q",
        job_type: "t",
        job_id: Uuid::new_v4(),
        attempts: 2,
        max_attempts: 5,
    };
    assert_eq!(job.attempts, 2);
    let op = WorkerOperationContext {
        app_name: "app",
        environment: "env",
        operation: "op",
    };
    assert_eq!(op.operation, "op");
}

#[test]
fn capture_paths_run_when_error_reporting_is_configured() {
    let _guard = init_error_reporting_with_config(ErrorReportingConfig {
        app_name: "nvbes-observability-tests",
        service_name: "nvbes-observability-tests",
        environment: "test",
        dsn: Some("https://public@127.0.0.1/1"),
        traces_sample_rate: 0.0,
    });
    assert!(is_error_reporting_configured());

    let error = io::Error::other("configured failure");
    capture_worker_job_error(
        &error,
        &WorkerJobContext {
            app_name: "email-worker",
            environment: "test",
            queue: "email",
            job_type: "send",
            job_id: Uuid::nil(),
            attempts: 2,
            max_attempts: 3,
        },
    );
    capture_worker_operation_error(
        &error,
        &WorkerOperationContext {
            app_name: "email-worker",
            environment: "test",
            operation: "dispatch",
        },
    );
}
