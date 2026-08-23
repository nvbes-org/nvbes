use nvbes_core::config::AppConfig;
use nvbes_observability::{
    capture_error_reporting_smoke, init_error_reporting_for_service, init_tracing_for_service,
    install_safe_panic_hook, start_continuous_profiling,
};

#[path = "identity.worker.state.rs"]
mod app;
#[path = "identity.grpc.pb.rs"]
mod grpc_pb;
#[path = "identity.worker.migrations.rs"]
mod migrations;
#[path = "identity.worker.rs"]
mod worker;

const IDENTITY_WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR";
const WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_WORKER_METRICS_BIND_ADDR";
const DEFAULT_IDENTITY_WORKER_METRICS_BIND_ADDR: &str = "127.0.0.1:4102";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arg1 = std::env::args().nth(1);
    if matches!(arg1.as_deref(), Some("run-audit-anchor")) {
        return worker::audit_anchor::run_standalone().await;
    }

    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;

    let _error_reporting_guard = init_error_reporting_for_service(&config, "identity-worker");
    install_safe_panic_hook();
    init_tracing_for_service(&config, "identity-worker");

    if matches!(arg1.as_deref(), Some("error-reporting-smoke")) {
        let result = capture_error_reporting_smoke(
            "identity-worker",
            &config.environment,
            "worker",
            config.sentry_dsn.is_some(),
        );
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(arg1.as_deref(), Some("migrate")) {
        tracing::info!("running identity database migrations with the dedicated migrator role");
        return migrations::run(&config).await;
    }

    let _profiling_guard =
        start_continuous_profiling(&config, "identity-worker").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;

    let state = app::AppState::bootstrap(&config, db).await?;

    if matches!(arg1.as_deref(), Some("run-housekeeping")) {
        tracing::info!("running identity-worker in Serverless Job mode: housekeeping");
        return worker::run_housekeeping_job(&state).await;
    }

    let metrics_bind_addr = identity_worker_metrics_bind_addr();
    let _metrics_server = nvbes_observability::start_metrics_server(
        &config,
        state.observability.clone(),
        &metrics_bind_addr,
    )
    .await?;

    tracing::info!(
        app = %config.app_name,
        environment = %config.environment,
        "starting nvbes Identity worker"
    );

    worker::run_loop_until_shutdown(state, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

fn identity_worker_metrics_bind_addr() -> String {
    std::env::var(IDENTITY_WORKER_METRICS_BIND_ADDR_ENV)
        .or_else(|_| std::env::var(WORKER_METRICS_BIND_ADDR_ENV))
        .unwrap_or_else(|_| DEFAULT_IDENTITY_WORKER_METRICS_BIND_ADDR.to_string())
}

#[cfg(test)]
#[path = "identity.worker.main.tests.rs"]
mod tests;
