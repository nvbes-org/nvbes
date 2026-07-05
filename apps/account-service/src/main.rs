use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    init_error_reporting_for_service, init_tracing, install_safe_panic_hook,
    start_continuous_profiling,
};
use std::net::SocketAddr;
use std::time::Duration;
use utoipa::OpenApi;

#[path = "identity.tools.beta.rs"]
mod beta_tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--export-openapi") {
        let doc = nvbes_identity_api::http::openapi::IdentityApiDoc::openapi();
        println!("{}", doc.to_json()?);
        return Ok(());
    }

    if let Some(command) = beta_tools::parse_cli_command(&args)? {
        beta_tools::run_cli_command(command).await?;
        return Ok(());
    }

    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    if login_protection_required(&config.environment) && !config.auth_pow_enabled {
        panic!(
            "NVBES_AUTH_POW_ENABLED=true is required outside development/test. Refusing to start with fail-open login protection."
        );
    }

    let _error_reporting_guard = init_error_reporting_for_service(&config, "identity-api");
    install_safe_panic_hook();
    init_tracing(&config);
    let _profiling_guard =
        start_continuous_profiling(&config, "identity-api").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    let state = nvbes_identity_api::app::AppState::bootstrap(&config, db).await?;
    let app = nvbes_identity_api::app::build_router(state);

    let http_addr: SocketAddr = format!("0.0.0.0:{}", config.api_port).parse()?;
    let http_listener = keep_alive::bind_listener_with_keepalive(http_addr, 4096)?;
    tracing::info!(addr = %http_addr, "Starting HTTP listener");

    if config.mtls_enabled {
        let mtls_addr: SocketAddr = format!("0.0.0.0:{}", config.mtls_port).parse()?;
        let mtls_acceptor = nvbes_core::tls::build_mtls_acceptor(&config)
            .await
            .map_err(anyhow::Error::msg)?;
        let mtls_app = app.clone();
        let mtls_handle = axum_server::Handle::new();

        let mtls_server =
            axum_server::bind_rustls(mtls_addr, mtls_acceptor).handle(mtls_handle.clone());

        tracing::info!(addr = %mtls_addr, "Starting mTLS listener");

        tokio::spawn(async move {
            if let Err(e) = mtls_server.serve(mtls_app.into_make_service()).await {
                tracing::error!(?e, "mTLS server error");
            }
        });

        axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

        mtls_handle.graceful_shutdown(Some(Duration::from_secs(30)));
    } else {
        tracing::info!(%http_addr, "Starting nvbes Identity API");
        axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    }

    Ok(())
}

fn login_protection_required(environment: &str) -> bool {
    !matches!(environment, "development" | "test")
}

#[cfg(test)]
mod tests {
    use super::login_protection_required;

    #[test]
    fn login_protection_is_not_required_for_local_envs() {
        assert!(!login_protection_required("development"));
        assert!(!login_protection_required("test"));
    }

    #[test]
    fn login_protection_is_required_for_deployed_envs() {
        assert!(login_protection_required("staging"));
        assert!(login_protection_required("production"));
    }
}
