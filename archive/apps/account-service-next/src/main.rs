use std::net::SocketAddr;

use nvbes_account_service::{app::AppState, config::AccountConfig, openapi::AccountApiDoc};
use sqlx::postgres::PgPoolOptions;
use utoipa::OpenApi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|argument| argument == "--export-openapi") {
        println!("{}", AccountApiDoc::openapi().to_pretty_json()?);
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nvbes_account_service=info,tower_http=info".into()),
        )
        .json()
        .init();

    let config = AccountConfig::from_env().map_err(anyhow::Error::msg)?;
    let db = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let state = AppState::bootstrap(config.clone(), db).await?;
    let app = nvbes_account_service::app::build_router(state);
    let address = SocketAddr::from(([0, 0, 0, 0], config.service_port));
    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!(
        %address,
        service_url = %config.service_base_url,
        identity_issuer = %config.identity_service_base_url,
        account_web = %config.account_web_base_url,
        "starting nvbes Account resource server"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
