use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    capture_error_reporting_smoke, init_error_reporting_for_service, init_tracing,
    install_safe_panic_hook, start_continuous_profiling,
};
use std::net::SocketAddr;

const BILLING_API_PORT_ENV: &str = "NVBES_BILLING_API_PORT";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;

    let _error_reporting_guard = init_error_reporting_for_service(&config, "billing-api");
    install_safe_panic_hook();
    init_tracing(&config);

    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("error-reporting-smoke")
    ) {
        let result = capture_error_reporting_smoke(
            "billing-api",
            &config.environment,
            "api",
            config.sentry_dsn.is_some(),
        );
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let _profiling_guard =
        start_continuous_profiling(&config, "billing-api").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;
    let state = nvbes_billing_api::app::BillingAppState::bootstrap(&config, db).await?;
    let app = nvbes_billing_api::app::build_router(state);

    let port = billing_api_port(config.api_port)?;
    let http_addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let http_listener = keep_alive::bind_listener_with_keepalive(http_addr, 4096)?;
    tracing::info!(addr = %http_addr, "Starting nvbes Billing API");

    axum::serve(
        http_listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;

    Ok(())
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
    use super::billing_api_port;

    #[test]
    fn billing_api_port_defaults_after_primary_api_port() {
        assert_eq!(billing_api_port(3000).unwrap(), 3020);
    }
}
