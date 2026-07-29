use std::net::SocketAddr;

use nvbes_core::config::AppConfig;
use nvbes_observability::{
    init_error_reporting_for_service, init_tracing, install_safe_panic_hook,
    start_continuous_profiling,
};
use utoipa::OpenApi;

const DEVELOPER_GRPC_PORT_ENV: &str = "NVBES_DEVELOPER_GRPC_PORT";
const DEVELOPER_HTTP_PORT_ENV: &str = "NVBES_DEVELOPER_HTTP_PORT";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|argument| argument == "--export-openapi") {
        let document = nvbes_developer_service::http::openapi::DeveloperApiDoc::openapi();
        println!("{}", document.to_json()?);
        return Ok(());
    }

    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;
    let _error_reporting_guard = init_error_reporting_for_service(&config, "developer-service");
    install_safe_panic_hook();
    init_tracing(&config);
    let _profiling_guard =
        start_continuous_profiling(&config, "developer-service").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;
    let state = nvbes_developer_service::app::DeveloperAppState::bootstrap(&config, db).await?;
    let http_addr: SocketAddr =
        format!("0.0.0.0:{}", developer_http_port(config.api_port)?).parse()?;
    let grpc_addr: SocketAddr =
        format!("0.0.0.0:{}", developer_grpc_port(config.api_port)?).parse()?;

    tracing::info!(addr = %http_addr, "Starting nvbes Developer HTTP API");
    tracing::info!(addr = %grpc_addr, "Starting nvbes Developer gRPC API");
    let http = nvbes_developer_service::http::serve(http_addr, state.clone(), shutdown_signal());
    let grpc = nvbes_developer_service::grpc::service::serve(grpc_addr, state, shutdown_signal());
    tokio::try_join!(async { http.await.map_err(anyhow::Error::from) }, async {
        grpc.await.map_err(anyhow::Error::from)
    })?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn developer_http_port(default_api_port: u16) -> anyhow::Result<u16> {
    service_port(DEVELOPER_HTTP_PORT_ENV, default_api_port, 40)
}

fn developer_grpc_port(default_api_port: u16) -> anyhow::Result<u16> {
    service_port(DEVELOPER_GRPC_PORT_ENV, default_api_port, 41)
}

fn service_port(env: &'static str, default_api_port: u16, offset: u16) -> anyhow::Result<u16> {
    match std::env::var(env) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{env} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(offset)
            .ok_or_else(|| anyhow::anyhow!("Default Developer service port overflowed")),
        Err(error) => Err(anyhow::anyhow!("{env} could not be read: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{developer_grpc_port, developer_http_port};

    #[test]
    fn developer_ports_default_after_primary_api_port() {
        assert_eq!(developer_http_port(3000).unwrap(), 3040);
        assert_eq!(developer_grpc_port(3000).unwrap(), 3041);
    }
}
