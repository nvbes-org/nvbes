use nvbes_observability::{
    ErrorReportingConfig, ErrorReportingGuard, ErrorReportingSmokeResult,
    capture_error_reporting_smoke, init_error_reporting_with_config,
};

pub const APP_NAME: &str = "nvbes-identity-service";

#[derive(Debug)]
pub struct ErrorReportingRuntimeConfig {
    pub(crate) environment: String,
    pub(crate) dsn: String,
    pub(crate) traces_sample_rate: f32,
}

impl ErrorReportingRuntimeConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let environment = required("NVBES_ENVIRONMENT")?;
        let dsn = required("SENTRY_DSN")?;
        anyhow::ensure!(dsn.starts_with("https://"), "SENTRY_DSN must use HTTPS");
        let traces_sample_rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
            .unwrap_or_else(|_| "0".to_owned())
            .parse::<f32>()?;
        anyhow::ensure!(
            (0.0..=1.0).contains(&traces_sample_rate),
            "SENTRY_TRACES_SAMPLE_RATE must be between 0 and 1"
        );
        Ok(Self {
            environment,
            dsn,
            traces_sample_rate,
        })
    }
}

pub fn init(config: &ErrorReportingRuntimeConfig) -> ErrorReportingGuard {
    init_error_reporting_with_config(ErrorReportingConfig {
        app_name: APP_NAME,
        service_name: APP_NAME,
        environment: &config.environment,
        dsn: Some(&config.dsn),
        traces_sample_rate: config.traces_sample_rate,
    })
}

pub fn smoke(config: &ErrorReportingRuntimeConfig) -> ErrorReportingSmokeResult {
    capture_error_reporting_smoke(APP_NAME, &config.environment, "serverless-job", true)
}

fn required(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

#[cfg(test)]
#[path = "identity.error_reporting.tests.rs"]
mod tests;
