use nvbes_core::config::AppConfig;
use nvbes_observability::{
    capture_error_reporting_smoke, init_error_reporting, init_tracing, install_safe_panic_hook,
    start_continuous_profiling,
};

#[path = "identity.worker.rs"]
mod worker;

pub use nvbes_identity_api::{app, domains, email, http};

const IDENTITY_WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR";
const WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_WORKER_METRICS_BIND_ADDR";
const DEFAULT_IDENTITY_WORKER_METRICS_BIND_ADDR: &str = "127.0.0.1:4102";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;

    let _error_reporting_guard = init_error_reporting(&config);
    install_safe_panic_hook();
    init_tracing(&config);

    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("error-reporting-smoke")
    ) {
        let result =
            capture_error_reporting_smoke("identity-worker", &config.environment, "worker", false);
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let _profiling_guard =
        start_continuous_profiling(&config, "identity-worker").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;

    nvbes_identity_api::database::run_migrations(&db).await?;

    let state = nvbes_identity_api::app::AppState::bootstrap(&config, db).await?;
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
        "starting nvbes Identity billing worker"
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
