use nvbes_email::proto::nvbes::email::v1::email_delivery_service_server::EmailDeliveryServiceServer;
use tokio::sync::watch;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

#[path = "email.worker.grpc.auth.rs"]
mod auth;
#[path = "email.worker.config.rs"]
mod config;
#[path = "email.worker.crypto.rs"]
mod crypto;
#[path = "email.worker.database.rs"]
mod database;
#[path = "email.worker.dispatch.attempt.rs"]
mod dispatch_attempt;
#[path = "email.worker.dispatch.db.rs"]
mod dispatch_db;
#[path = "email.worker.dispatcher.rs"]
mod dispatcher;
#[path = "email.worker.grpc.service.rs"]
mod grpc_service;
#[path = "email.worker.health.rs"]
mod health;
#[path = "email.worker.state.rs"]
mod state;
#[path = "email.worker.webhook.rs"]
mod webhook;
#[path = "email.worker.webhook.db.rs"]
mod webhook_db;
#[path = "email.worker.webhook.verify.rs"]
mod webhook_verify;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = config::EmailWorkerConfig::from_env()?;
    let db = database::connect(&config.database_url).await?;
    database::migrate(&db).await?;

    if matches!(std::env::args().nth(1).as_deref(), Some("migrate")) {
        tracing::info!("email database migrations applied");
        return Ok(());
    }

    let state = state::EmailWorkerState::new(config, db)?;
    let http_listener = tokio::net::TcpListener::bind(state.config.http_bind_addr).await?;
    let grpc_addr = state.config.grpc_bind_addr;
    let email_grpc = grpc_service::EmailDeliveryGrpcService::new(state.clone());
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<EmailDeliveryServiceServer<grpc_service::EmailDeliveryGrpcService>>()
        .await;
    let grpc = EmailDeliveryServiceServer::new(email_grpc);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let dispatcher = tokio::spawn(dispatcher::run(state.clone(), shutdown_rx.clone()));
    let http_router = health::router(state.clone()).merge(webhook::router(state.clone()));
    let http_server = axum::serve(http_listener, http_router)
        .with_graceful_shutdown(server_shutdown(shutdown_rx.clone()));
    let grpc_server = Server::builder()
        .add_service(health_service)
        .add_service(grpc)
        .serve_with_shutdown(grpc_addr, server_shutdown(shutdown_rx));

    tracing::info!(
        http_addr = %state.config.http_bind_addr,
        grpc_addr = %grpc_addr,
        environment = %state.config.environment,
        provider = provider_name(&state.config.provider),
        from_email = %redacted_sender(&state.config.from_email),
        from_name = %state.config.from_name,
        reply_to_configured = state.config.reply_to.is_some(),
        "starting nvbes email worker"
    );

    tokio::select! {
        result = http_server => result?,
        result = grpc_server => result?,
        _ = process_shutdown_signal() => {},
    }
    let _ = shutdown_tx.send(true);
    dispatcher.await?;
    Ok(())
}

async fn server_shutdown(mut shutdown: watch::Receiver<bool>) {
    while !*shutdown.borrow() {
        if shutdown.changed().await.is_err() {
            break;
        }
    }
}

#[cfg(unix)]
async fn process_shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler must register");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}

#[cfg(not(unix))]
async fn process_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn provider_name(provider: &config::ProviderConfig) -> &'static str {
    match provider {
        config::ProviderConfig::Mock => "mock",
        config::ProviderConfig::Smtp(_) => "smtp",
        config::ProviderConfig::TestCapture(_) => "test-capture",
        config::ProviderConfig::Scaleway(_) => "scaleway",
    }
}

fn redacted_sender(sender: &str) -> String {
    sender
        .split_once('@')
        .map(|(_, domain)| format!("***@{domain}"))
        .unwrap_or_else(|| "***".to_string())
}
