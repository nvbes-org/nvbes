#[path = "drive.app.rs"]
mod app;
#[path = "drive.db.mod.rs"]
mod db;
#[path = "drive.domains.mod.rs"]
mod domains;
#[path = "drive.http.mod.rs"]
mod http;

use std::net::SocketAddr;
use std::time::Duration;

use app::build_app;
use db::Database;
use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    init_error_reporting, init_tracing, install_safe_panic_hook, start_continuous_profiling,
};
use utoipa::OpenApi;

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
        database.migrate().await?;
        tracing::info!("database migrations applied");
        return Ok(());
    }

    database.migrate().await?;
    let _profiling_guard =
        start_continuous_profiling(&config, "drive-api").map_err(anyhow::Error::msg)?;

    let app = build_app(config.clone(), database).await?;

    let http_address = SocketAddr::from(([0, 0, 0, 0], config.api_port));
    let http_listener = keep_alive::bind_listener_with_keepalive(http_address, 4096)?;

    let app_name = config.app_name.clone();
    let environment = config.environment.clone();

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
        tracing::info!(
            app = %app_name,
            environment = %environment,
            database_max_connections = config.database_max_connections,
            address = %http_address,
            "starting nvbes API"
        );

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
