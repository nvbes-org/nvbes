use std::net::SocketAddr;
use std::sync::Arc;

use nvbes_platform::{
    cockpit_auth::OperatorAuthPolicy,
    cockpit_finops::FinOpsMonitor,
    cockpit_health::HealthAggregator,
    cockpit_server::{create_platform_cockpit_router, PlatformCockpitState},
};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("NVBES_PLATFORM_OPERATIONS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8084);

    let environment = std::env::var("NVBES_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    let operator_token = std::env::var("NVBES_PLATFORM_OPERATIONS_TOKEN").unwrap_or_else(|_| {
        if environment == "production" {
            panic!("NVBES_PLATFORM_OPERATIONS_TOKEN is mandatory in production");
        }
        warn!("NVBES_PLATFORM_OPERATIONS_TOKEN not set; defaulting to local-dev-token");
        "local-dev-token".to_string()
    });

    let state = PlatformCockpitState {
        environment: environment.clone(),
        auth_policy: Arc::new(OperatorAuthPolicy::new(operator_token)),
        health_aggregator: Arc::new(HealthAggregator::default()),
        finops_monitor: Arc::new(FinOpsMonitor::default()),
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
