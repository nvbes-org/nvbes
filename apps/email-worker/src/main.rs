use nvbes_email::proto::nvbes::email::v1::{
    email_delivery_service_server::EmailDeliveryServiceServer,
    email_operations_service_server::EmailOperationsServiceServer,
};
use tokio::sync::watch;

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
#[path = "email.worker.dispatcher.maintenance.rs"]
mod dispatcher_maintenance;
#[path = "email.worker.metrics.rs"]
mod email_metrics;
#[path = "email.worker.error_reporting.rs"]
mod error_reporting;
#[path = "email.worker.grpc.operations.rs"]
mod grpc_operations;
#[path = "email.worker.grpc.service.rs"]
mod grpc_service;
#[path = "email.worker.health.rs"]
mod health;
#[path = "email.worker.operations.actions.rs"]
mod operations_actions;
#[path = "email.worker.operations.privacy.rs"]
mod operations_privacy;
#[path = "email.worker.operations.snapshot.rs"]
mod operations_snapshot;
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
    let command: Vec<String> = std::env::args().skip(1).collect();
    if matches!(command.as_slice(), [action] if action == "deployment-bootstrap") {
        return run_deployment_bootstrap().await;
    }

    let config = config::EmailWorkerConfig::from_env()?;
    let _error_reporting_guard = error_reporting::init(&config);
    nvbes_observability::install_safe_panic_hook();
    nvbes_observability::init_tracing_with_config(
        nvbes_observability::TracingConfig {
            environment: &config.environment,
            otlp_endpoint: config.otlp_endpoint.as_deref(),
            otlp_authorization_header: config.otlp_authorization_header.as_deref(),
        },
        error_reporting::APP_NAME,
    );
    if matches!(command.as_slice(), [action] if action == "error-reporting-smoke") {
        println!(
            "{}",
            serde_json::to_string(&error_reporting::smoke(&config))?
        );
        return Ok(());
    }

    let result = run(&config, &command).await;
    if let Err(error) = result.as_ref() {
        error_reporting::capture_operation(&config, "runtime", error.as_ref());
    }
    result
}

async fn run_deployment_bootstrap() -> anyhow::Result<()> {
    let bind_addr: std::net::SocketAddr = std::env::var("NVBES_EMAIL_HTTP_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3040".to_string())
        .parse()
        .map_err(|error| anyhow::anyhow!("NVBES_EMAIL_HTTP_BIND_ADDR is invalid: {error}"))?;
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let router = axum::Router::new().route(
        "/health/live",
        axum::routing::get(|| async { axum::http::StatusCode::NO_CONTENT }),
    );

    axum::serve(listener, router)
        .with_graceful_shutdown(process_shutdown_signal())
        .await?;
    Ok(())
}

async fn run(config: &config::EmailWorkerConfig, command: &[String]) -> anyhow::Result<()> {
    let db = database::connect(&config.database_url).await?;
    database::migrate(&db).await?;

    match command {
        [] => {}
        [action] if action == "migrate" => {
            tracing::info!("email database migrations applied");
            return Ok(());
        }
        [action, message_id, actor, reason] if action == "release-suppression" => {
            release_suppression(&db, message_id, actor, reason).await?;
            return Ok(());
        }
        _ => anyhow::bail!(
            "usage: nvbes-email-worker [migrate|error-reporting-smoke|release-suppression <message-id> <actor> <reason>]"
        ),
    }

    let state = state::EmailWorkerState::new(config.clone(), db)?;
    let http_listener = tokio::net::TcpListener::bind(state.config.http_bind_addr).await?;
    let grpc_addr = state.config.grpc_bind_addr;
    let email_grpc = grpc_service::EmailDeliveryGrpcService::new(state.clone());
    let operations_grpc = grpc_operations::EmailOperationsGrpcService::new(state.clone());
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<EmailDeliveryServiceServer<grpc_service::EmailDeliveryGrpcService>>()
        .await;
    health_reporter
        .set_serving::<EmailOperationsServiceServer<grpc_operations::EmailOperationsGrpcService>>()
        .await;
    let grpc = EmailDeliveryServiceServer::new(email_grpc);
    let operations = EmailOperationsServiceServer::new(operations_grpc);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let dispatcher = tokio::spawn(dispatcher::run(state.clone(), shutdown_rx.clone()));
    let http_router = health::router(state.clone())
        .merge(email_metrics::router(state.clone()))
        .merge(webhook::router(state.clone()));
    let application = tonic::service::Routes::from(http_router)
        .add_service(health_service)
        .add_service(grpc)
        .add_service(operations)
        .into_axum_router();
    let server = axum::serve(http_listener, application)
        .with_graceful_shutdown(server_shutdown(shutdown_rx));

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
        result = server => result?,
        _ = process_shutdown_signal() => {},
    }
    let _ = shutdown_tx.send(true);
    dispatcher.await?;
    Ok(())
}

async fn release_suppression(
    db: &sqlx::PgPool,
    message_id: &str,
    actor: &str,
    reason: &str,
) -> anyhow::Result<()> {
    let message_id = message_id
        .parse::<uuid::Uuid>()
        .map_err(|_| anyhow::anyhow!("release-suppression message-id must be a UUID"))?;
    if actor.trim().is_empty()
        || actor.len() > 200
        || !actor
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
        || reason.trim().is_empty()
        || reason.len() > 500
        || reason.contains(['\r', '\n'])
    {
        anyhow::bail!("release-suppression actor or reason is invalid");
    }
    if !database::release_suppression_by_message(db, message_id, actor, reason).await? {
        anyhow::bail!("no active suppression was found for the message recipient");
    }
    tracing::info!(%message_id, actor, "email suppression released through audited procedure");
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
    provider.label()
}

fn redacted_sender(sender: &str) -> String {
    sender
        .split_once('@')
        .map(|(_, domain)| format!("***@{domain}"))
        .unwrap_or_else(|| "***".to_string())
}
