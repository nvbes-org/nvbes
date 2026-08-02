use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    capture_error_reporting_smoke, init_error_reporting_for_service, init_tracing,
    install_safe_panic_hook, start_continuous_profiling,
};
use std::net::SocketAddr;
use tokio::sync::broadcast;

const BILLING_API_PORT_ENV: &str = "NVBES_BILLING_SERVICE_PORT";
const BILLING_GRPC_PORT_ENV: &str = "NVBES_BILLING_GRPC_PORT";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;
    let command = std::env::args().nth(1);

    let _error_reporting_guard = init_error_reporting_for_service(&config, "billing-service");
    install_safe_panic_hook();
    init_tracing(&config);

    if matches!(command.as_deref(), Some("error-reporting-smoke")) {
        let result = capture_error_reporting_smoke(
            "billing-service",
            &config.environment,
            "api",
            config.sentry_dsn.is_some(),
        );
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let _profiling_guard =
        start_continuous_profiling(&config, "billing-service").map_err(anyhow::Error::msg)?;

    let db = nvbes_billing_service::database::connect_pool(&config).await?;
    nvbes_billing_service::database::run_migrations(&db).await?;
    if matches!(command.as_deref(), Some("migrate")) {
        tracing::info!("Billing database migrations applied");
        return Ok(());
    }
    let state = nvbes_billing_service::app::BillingAppState::bootstrap(&config, db).await?;
    let app = nvbes_billing_service::app::build_router(state.clone());

    let port = billing_api_port(config.api_port)?;
    let http_addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let http_listener = keep_alive::bind_listener_with_keepalive(http_addr, 4096)?;
    tracing::info!(addr = %http_addr, "Starting nvbes billing-service HTTP");

    let grpc_port = billing_grpc_port(config.api_port)?;
    let grpc_addr: SocketAddr = format!("0.0.0.0:{grpc_port}").parse()?;
    tracing::info!(addr = %grpc_addr, "Starting nvbes Billing gRPC API");

    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_signal_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_signal_tx.send(());
    });

    let http_server = axum::serve(
        http_listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_tx.subscribe()));
    let grpc_server = nvbes_billing_service::grpc::service::serve(
        grpc_addr,
        state,
        shutdown_signal(shutdown_tx.subscribe()),
    );

    tokio::try_join!(
        async { http_server.await.map_err(anyhow::Error::from) },
        async { grpc_server.await.map_err(anyhow::Error::from) },
    )?;

    Ok(())
}

async fn shutdown_signal(mut shutdown_rx: broadcast::Receiver<()>) {
    let _ = shutdown_rx.recv().await;
}

fn billing_grpc_port(default_api_port: u16) -> anyhow::Result<u16> {
    match std::env::var(BILLING_GRPC_PORT_ENV) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{BILLING_GRPC_PORT_ENV} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(21)
            .ok_or_else(|| anyhow::anyhow!("Default billing gRPC port overflowed")),
        Err(error) => Err(anyhow::anyhow!(
            "{BILLING_GRPC_PORT_ENV} could not be read: {error}"
        )),
    }
}

fn billing_api_port(default_api_port: u16) -> anyhow::Result<u16> {
    match std::env::var(BILLING_API_PORT_ENV) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{BILLING_API_PORT_ENV} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(20)
            .ok_or_else(|| anyhow::anyhow!("Default billing API port overflowed")),
        Err(error) => Err(anyhow::anyhow!(
            "{BILLING_API_PORT_ENV} could not be read: {error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{billing_api_port, billing_grpc_port};

    #[test]
    fn billing_api_port_defaults_after_primary_api_port() {
        assert_eq!(billing_api_port(3000).unwrap(), 3020);
    }

    #[test]
    fn billing_grpc_port_defaults_after_primary_api_port() {
        assert_eq!(billing_grpc_port(3000).unwrap(), 3021);
    }
}
