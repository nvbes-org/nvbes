use anyhow::Context;
use nvbes_worker::{Command, WorkerConfig, cli, server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = cli::parse(&args)?;

    nvbes_observability::install_safe_panic_hook();
    let config = WorkerConfig::from_env().context("failed to load Worker configuration")?;

    nvbes_observability::init_tracing_with_config(
        nvbes_observability::TracingConfig {
            environment: &config.environment,
            otlp_endpoint: config.otlp_endpoint.as_deref(),
            otlp_authorization_header: config.otlp_authorization_header.as_deref(),
            protocol: nvbes_observability::OtlpProtocol::Http,
        },
        "nvbes-worker",
    );

    let _error_reporting_guard = config.sentry_dsn.as_ref().map(|dsn| {
        nvbes_observability::init_error_reporting_with_config(
            nvbes_observability::ErrorReportingConfig {
                app_name: "nvbes-worker",
                service_name: "nvbes-worker",
                environment: &config.environment,
                dsn: Some(dsn.as_str()),
                traces_sample_rate: config.sentry_traces_sample_rate,
            },
        )
    });

    let result = match command {
        Command::Serve => server::serve(config).await,
        Command::Run(task) => server::run_once(config, task).await,
    };
    nvbes_observability::shutdown_tracing();
    let _ = nvbes_observability::flush_error_reporting(std::time::Duration::from_secs(1));
    result
}
