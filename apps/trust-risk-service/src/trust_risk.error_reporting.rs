use std::error::Error;

use nvbes_observability::{
    WorkerOperationContext, capture_error_reporting_smoke, capture_worker_operation_error,
};

use crate::config::TrustRiskConfig;

pub const APP_NAME: &str = "nvbes-trust-risk-service";

pub fn smoke(config: &TrustRiskConfig) -> nvbes_observability::ErrorReportingSmokeResult {
    capture_error_reporting_smoke(
        APP_NAME,
        &config.environment,
        "service",
        config.sentry_dsn.is_some(),
    )
}

pub fn capture_operation(
    config: &TrustRiskConfig,
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
