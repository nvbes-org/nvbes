use std::net::SocketAddr;
use std::sync::Arc;

use nvbes_platform::{
    cockpit_auth::OperatorAuthPolicy,
    cockpit_server::{PlatformCockpitState, create_platform_cockpit_router},
    operations_context::{ContextClient, ServiceEndpoint},
    operations_db::MIGRATOR,
};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("NVBES_PLATFORM_OPERATIONS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8084);

    let environment =
        std::env::var("NVBES_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&std::env::var("NVBES_PLATFORM_OPERATIONS_DATABASE_URL")?)
        .await?;
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        MIGRATOR.run(&db).await?;
        return Ok(());
    }
    let auth_policy = OperatorAuthPolicy::from_rsa_pem(
        std::env::var("NVBES_PLATFORM_OPERATIONS_PUBLIC_KEY_PEM")?.as_bytes(),
        &std::env::var("NVBES_PLATFORM_OPERATIONS_ISSUER")?,
        "platform-operations",
    )?;
    let endpoints: Vec<ServiceEndpoint> = serde_json::from_str(
        &std::env::var("NVBES_PLATFORM_OPERATIONS_SERVICES").unwrap_or_else(|_| "[]".into()),
    )?;
    let context = ContextClient::new(endpoints, environment == "production")
        .map_err(std::io::Error::other)?;

    let state = PlatformCockpitState {
        environment: environment.clone(),
        auth_policy: Arc::new(auth_policy),
        db,
        context: Arc::new(context),
    };

    let router = create_platform_cockpit_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    info!(
        "nvbes Platform Operations Cockpit started on {} in {} mode",
        addr, environment
    );

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("nvbes Platform Operations Cockpit stopped cleanly");
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
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
