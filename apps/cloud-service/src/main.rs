#[path = "drive.app.rs"]
mod app;
pub use nvbes_product_cloud::db;
#[path = "drive.domains.mod.rs"]
mod domains;
#[path = "drive.grpc.mod.rs"]
mod grpc;
#[path = "drive.http.mod.rs"]
mod http;
#[cfg(test)]
#[path = "drive.test_support.db.rs"]
mod test_support;

use std::net::SocketAddr;
use std::time::Duration;

use app::{build_app_state, build_router};
use db::Database;
use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    init_error_reporting, init_tracing, install_safe_panic_hook, start_continuous_profiling,
};
use tokio::sync::broadcast;
use utoipa::OpenApi;

const CLOUD_GRPC_PORT_ENV: &str = "NVBES_CLOUD_GRPC_PORT";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--export-openapi") {
        let doc = http::openapi::DriveApiDoc::openapi();
        println!("{}", doc.to_json()?);
        return Ok(());
    }

    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    let _error_reporting_guard = init_error_reporting(&config);
    install_safe_panic_hook();
    init_tracing(&config);

    let command = std::env::args().nth(1);
    let database = Database::connect(&config).await?;

    if matches!(command.as_deref(), Some("migrate")) {
        run_migrations(&database).await?;
        tracing::info!("database migrations applied");
        return Ok(());
    }

    run_migrations(&database).await?;
    let _profiling_guard =
        start_continuous_profiling(&config, "cloud-service").map_err(anyhow::Error::msg)?;

    let state = build_app_state(config.clone(), database).await?;
    let app = build_router(state.clone());

    let http_address = SocketAddr::from(([0, 0, 0, 0], config.api_port));
    let http_listener = keep_alive::bind_listener_with_keepalive(http_address, 4096)?;
    let grpc_port = cloud_grpc_port(config.api_port)?;
    let grpc_address: SocketAddr = format!("0.0.0.0:{grpc_port}").parse()?;

    let app_name = config.app_name.clone();
    let environment = config.environment.clone();
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

        let mtls_server =
            axum_server::bind_rustls(mtls_addr, mtls_acceptor).handle(mtls_handle.clone());

        tracing::info!(addr = %mtls_addr, "Starting mTLS listener");

        tokio::spawn(async move {
            if let Err(e) = mtls_server.serve(mtls_app.into_make_service()).await {
                tracing::error!(?e, "mTLS server error");
            }
        });

        tracing::info!(
            app = %app_name,
            environment = %environment,
            database_max_connections = config.database_max_connections,
            address = %http_address,
            "starting nvbes API"
        );

        let http_server = axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(shutdown_tx.subscribe()));
        let grpc_server = grpc::service::serve(
            grpc_address,
            state,
            shutdown_signal(shutdown_tx.subscribe()),
        );

        tokio::try_join!(
            async { http_server.await.map_err(anyhow::Error::from) },
            async { grpc_server.await.map_err(anyhow::Error::from) },
        )?;

        mtls_handle.graceful_shutdown(Some(Duration::from_secs(30)));
    } else {
        tracing::info!(
            app = %app_name,
            environment = %environment,
            database_max_connections = config.database_max_connections,
            address = %http_address,
            grpc_address = %grpc_address,
            "starting nvbes API"
        );

        let http_server = axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(shutdown_tx.subscribe()));
        let grpc_server = grpc::service::serve(
            grpc_address,
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

fn cloud_grpc_port(default_api_port: u16) -> anyhow::Result<u16> {
    match std::env::var(CLOUD_GRPC_PORT_ENV) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{CLOUD_GRPC_PORT_ENV} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Default Cloud gRPC port overflowed")),
        Err(error) => Err(anyhow::anyhow!(
            "{CLOUD_GRPC_PORT_ENV} could not be read: {error}"
        )),
    }
}

async fn run_migrations(database: &Database) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(&**database).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cloud_grpc_port;

    #[test]
    fn cloud_grpc_port_defaults_after_primary_api_port() {
        assert_eq!(cloud_grpc_port(3000).unwrap(), 3001);
    }
}
