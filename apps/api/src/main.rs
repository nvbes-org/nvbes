use anyhow::Context;
use nvbes_api::{ApiConfig, run_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nvbes_observability::install_safe_panic_hook();

    let config = ApiConfig::from_env().context("failed to load API configuration")?;

    nvbes_observability::init_tracing_with_config(
        nvbes_observability::TracingConfig {
            environment: &config.environment,
            otlp_endpoint: config.otlp_endpoint.as_deref(),
            otlp_authorization_header: config.otlp_authorization_header.as_deref(),
            protocol: nvbes_observability::OtlpProtocol::Http,
        },
        "nvbes-api",
    );

    let _error_reporting_guard = config.sentry_dsn.as_ref().map(|dsn| {
        nvbes_observability::init_error_reporting_with_config(
            nvbes_observability::ErrorReportingConfig {
                app_name: "nvbes-api",
                service_name: "nvbes-api",
                environment: &config.environment,
                dsn: Some(dsn.as_str()),
                traces_sample_rate: config.sentry_traces_sample_rate,
            },
        )
    });

    run_server(config).await
}
