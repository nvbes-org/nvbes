use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    init_error_reporting_for_service, init_tracing_for_service, install_safe_panic_hook,
    start_continuous_profiling,
};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::broadcast;
use utoipa::OpenApi;

#[path = "identity.tools.beta.rs"]
mod beta_tools;

const IDENTITY_GRPC_PORT_ENV: &str = "NVBES_IDENTITY_GRPC_PORT";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--export-openapi") {
        let doc = nvbes_account_service::http::openapi::IdentityApiDoc::openapi();
        println!("{}", doc.to_json()?);
        return Ok(());
    }

    if let Some(command) = beta_tools::parse_cli_command(&args)? {
        beta_tools::run_cli_command(command).await?;
        return Ok(());
    }

    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;
    if login_protection_required(&config.environment) && !config.auth_pow_enabled {
        panic!(
            "NVBES_AUTH_POW_ENABLED=true is required outside development/test. Refusing to start with fail-open login protection."
        );
    }

    let _error_reporting_guard = init_error_reporting_for_service(&config, "account-service");
    install_safe_panic_hook();
    init_tracing_for_service(&config, "account-service");
    let _profiling_guard =
        start_continuous_profiling(&config, "account-service").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    let state = nvbes_account_service::app::AppState::bootstrap(&config, db).await?;
    nvbes_account_service::domains::oauth::security_events::start_dispatcher(
        state.db.clone(),
        state.jwt.clone(),
    );
    let app = nvbes_account_service::app::build_router(state.clone());

    let http_addr: SocketAddr = format!("0.0.0.0:{}", config.api_port).parse()?;
    let http_listener = keep_alive::bind_listener_with_keepalive(http_addr, 4096)?;
    let grpc_addr: SocketAddr =
        format!("0.0.0.0:{}", account_grpc_port(config.api_port)?).parse()?;
    tracing::info!(addr = %http_addr, "Starting HTTP listener");
    tracing::info!(addr = %grpc_addr, "Starting internal Identity gRPC listener");

    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_signal_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_signal_tx.send(());
    });

    if config.mtls_enabled {
        let mtls_addr: SocketAddr = format!("0.0.0.0:{}", config.mtls_port).parse()?;
        let mtls_acceptor = nvbes_core::tls::build_mtls_acceptor(&config)
            .await
            .map_err(anyhow::Error::msg)?;
        let mtls_app = app.clone();
        let mtls_handle = axum_server::Handle::new();

        let mtls_server = axum_server::Server::bind(mtls_addr)
            .acceptor(
                nvbes_account_service::http::mtls::PeerCertificateAcceptor::new(mtls_acceptor),
            )
            .handle(mtls_handle.clone());

        tracing::info!(addr = %mtls_addr, "Starting mTLS listener");

        tokio::spawn(async move {
            if let Err(e) = mtls_server.serve(mtls_app.into_make_service()).await {
                tracing::error!(?e, "mTLS server error");
            }
        });

        let http_server = axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(shutdown_tx.subscribe()));
        let grpc_server = nvbes_account_service::grpc::service::serve(
            grpc_addr,
            state,
            shutdown_signal(shutdown_tx.subscribe()),
        );
        tokio::try_join!(
            async { http_server.await.map_err(anyhow::Error::from) },
            async { grpc_server.await.map_err(anyhow::Error::from) },
        )?;

        mtls_handle.graceful_shutdown(Some(Duration::from_secs(30)));
    } else {
        tracing::info!(%http_addr, "Starting nvbes Account Service");
        let http_server = axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(shutdown_tx.subscribe()));
        let grpc_server = nvbes_account_service::grpc::service::serve(
            grpc_addr,
            state,
            shutdown_signal(shutdown_tx.subscribe()),
        );
        tokio::try_join!(
            async { http_server.await.map_err(anyhow::Error::from) },
            async { grpc_server.await.map_err(anyhow::Error::from) },
        )?;
    }

    Ok(())
}

async fn shutdown_signal(mut shutdown_rx: broadcast::Receiver<()>) {
    let _ = shutdown_rx.recv().await;
}

fn account_grpc_port(default_api_port: u16) -> anyhow::Result<u16> {
    match std::env::var(IDENTITY_GRPC_PORT_ENV) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{IDENTITY_GRPC_PORT_ENV} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(10)
            .ok_or_else(|| anyhow::anyhow!("Default Account gRPC port overflowed")),
        Err(error) => Err(anyhow::anyhow!(
            "{IDENTITY_GRPC_PORT_ENV} could not be read: {error}"
        )),
    }
}

fn login_protection_required(environment: &str) -> bool {
    !matches!(environment, "development" | "test")
}

#[cfg(test)]
mod tests {
    use super::{account_grpc_port, login_protection_required};

    #[test]
    fn account_grpc_port_defaults_after_primary_api_port() {
        assert_eq!(account_grpc_port(4000).unwrap(), 4010);
    }

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
