use std::net::SocketAddr;

use nvbes_core::config::AppConfig;
use nvbes_observability::{
    init_error_reporting_for_service, init_tracing, install_safe_panic_hook,
    start_continuous_profiling,
};

const ENTERPRISE_GRPC_PORT_ENV: &str = "NVBES_ENTERPRISE_GRPC_PORT";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;
    let _error_reporting_guard = init_error_reporting_for_service(&config, "enterprise-service");
    install_safe_panic_hook();
    init_tracing(&config);
    let _profiling_guard =
        start_continuous_profiling(&config, "enterprise-service").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;
    let state = nvbes_enterprise_service::app::EnterpriseAppState::bootstrap(&config, db).await?;
    let grpc_authenticator =
        nvbes_enterprise_service::grpc::auth::EnterpriseGrpcAuthenticator::from_env(
            &config.environment,
        )
        .map_err(anyhow::Error::msg)?;
    let grpc_addr: SocketAddr =
        format!("0.0.0.0:{}", enterprise_grpc_port(config.api_port)?).parse()?;

    tracing::info!(addr = %grpc_addr, "Starting nvbes Enterprise gRPC API");
    nvbes_enterprise_service::grpc::service::serve(grpc_addr, state, grpc_authenticator, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;

    Ok(())
}

fn enterprise_grpc_port(default_api_port: u16) -> anyhow::Result<u16> {
    match std::env::var(ENTERPRISE_GRPC_PORT_ENV) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{ENTERPRISE_GRPC_PORT_ENV} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(31)
            .ok_or_else(|| anyhow::anyhow!("Default Enterprise gRPC port overflowed")),
        Err(error) => Err(anyhow::anyhow!(
            "{ENTERPRISE_GRPC_PORT_ENV} could not be read: {error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::enterprise_grpc_port;

    #[test]
    fn enterprise_grpc_port_defaults_after_primary_api_port() {
        assert_eq!(enterprise_grpc_port(3000).unwrap(), 3031);
    }
}
