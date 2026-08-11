use nvbes_email::proto::nvbes::email::v1::{
    email_delivery_service_server::EmailDeliveryServiceServer,
    email_operations_service_server::EmailOperationsServiceServer,
};
use tokio::sync::watch;
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
#[path = "email.worker.metrics.rs"]
mod email_metrics;
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
#[path = "email.worker.queue.rs"]
mod queue;
#[path = "email.worker.queue.trigger.rs"]
mod queue_trigger;
#[path = "email.worker.retention.rs"]
mod retention;
#[path = "email.worker.state.rs"]
mod state;
#[path = "email.worker.webhook.rs"]
mod webhook;
#[path = "email.worker.webhook.db.rs"]
mod webhook_db;
#[path = "email.worker.webhook.verify.rs"]
mod webhook_verify;

#[cfg(test)]
#[path = "email.worker.test_support.rs"]
mod test_support;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let command: Vec<String> = std::env::args().skip(1).collect();
    if matches!(command.as_slice(), [action] if action == "migrate") {
        let db = database::connect(&config::database_url_from_env()?).await?;
        database::migrate(&db).await?;
        tracing::info!("email database migrations applied");
        return Ok(());
    }

    let config = config::EmailWorkerConfig::from_env()?;
    match command.as_slice() {
        [] => {}
        [action, message_id, actor, reason] if action == "release-suppression" => {
            let db = database::connect(&config.database_url).await?;
            release_suppression(&db, message_id, actor, reason).await?;
            return Ok(());
        }
        _ => anyhow::bail!(
            "usage: nvbes-email-worker [migrate|release-suppression <message-id> <actor> <reason>]"
        ),
    }

    let db = database::connect_lazy(&config.database_url)?;
    let state = state::EmailWorkerState::new(config, db)?;
    let http_listener = tokio::net::TcpListener::bind(state.config.http_bind_addr).await?;
    let email_grpc = grpc_service::EmailDeliveryGrpcService::new(state.clone());
    let operations_grpc = grpc_operations::EmailOperationsGrpcService::new(state.clone());
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<EmailDeliveryServiceServer<grpc_service::EmailDeliveryGrpcService>>()
        .await;
    health_reporter
        .set_serving::<EmailOperationsServiceServer<grpc_operations::EmailOperationsGrpcService>>()
        .await;
    let grpc = grpc_service::delivery_server(email_grpc);
    let operations = EmailOperationsServiceServer::new(operations_grpc);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let local_dispatcher = if state.config.runtime_role == config::RuntimeRole::All {
        state.take_local_dispatch_receiver().map(|receiver| {
            tokio::spawn(dispatcher::run_local(
                state.clone(),
                receiver,
                shutdown_rx.clone(),
            ))
        })
    } else {
        None
    };
    let http_router = health::router(state.clone());
    let http_router = match state.config.runtime_role {
        config::RuntimeRole::All => http_router
            .merge(email_metrics::router(state.clone()))
            .merge(webhook::router(state.clone()))
            .merge(queue_trigger::router(state.clone()))
            .merge(retention::router(state.clone())),
        config::RuntimeRole::Ingress => http_router.merge(webhook::router(state.clone())),
        config::RuntimeRole::Dispatch => http_router
            .merge(email_metrics::router(state.clone()))
            .merge(queue_trigger::router(state.clone()))
            .merge(retention::router(state.clone())),
    };
    let app = tonic::service::Routes::from(http_router)
        .add_service(health_service)
        .add_service(grpc)
        .add_service(operations)
        .into_axum_router();
    let server = axum::serve(http_listener, app)
        .with_graceful_shutdown(server_shutdown(shutdown_rx.clone()));

    tracing::info!(
        bind_addr = %state.config.http_bind_addr,
        environment = %state.config.environment,
        provider = provider_name(&state.config.provider),
        runtime_role = ?state.config.runtime_role,
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
    if let Some(dispatcher) = local_dispatcher {
        dispatcher.await?;
    }
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

#[cfg(test)]
#[path = "email.worker.main.tests.rs"]
mod tests;
