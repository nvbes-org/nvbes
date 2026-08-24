#[path = "trust_risk.app.rs"]
mod app;
#[path = "trust_risk.assessment.db.rs"]
mod assessment_db;
#[path = "trust_risk.assessment.grpc.rs"]
mod assessment_grpc;
#[path = "trust_risk.audit.rs"]
mod audit;
#[path = "trust_risk.auth.rs"]
mod auth;
#[path = "trust_risk.config.rs"]
mod config;
#[path = "trust_risk.database.rs"]
mod database;
#[path = "trust_risk.health.rs"]
mod health;
#[path = "trust_risk.ingress.db.rs"]
mod ingress_db;
#[path = "trust_risk.ingress.grpc.rs"]
mod ingress_grpc;
#[path = "trust_risk.labels.db.rs"]
mod labels_db;
#[path = "trust_risk.labels.grpc.rs"]
mod labels_grpc;
#[path = "trust_risk.operations.grpc.rs"]
mod operations_grpc;
#[path = "trust_risk.operations.types.rs"]
mod operations_types;
#[path = "trust_risk.projection.rs"]
mod projection;
#[path = "trust_risk.projection.db.rs"]
mod projection_db;
#[path = "trust_risk.retention.rs"]
mod retention;
#[path = "trust_risk.review.db.rs"]
mod review_db;
#[path = "trust_risk.metrics.rs"]
mod risk_metrics;
#[path = "trust_risk.rules.db.rs"]
mod rules_db;

use nvbes_trust_risk::proto::nvbes::trust_risk::v1::{
    trust_risk_assessment_service_server::TrustRiskAssessmentServiceServer,
    trust_risk_label_service_server::TrustRiskLabelServiceServer,
    trust_risk_operations_service_server::TrustRiskOperationsServiceServer,
    trust_risk_signal_service_server::TrustRiskSignalServiceServer,
};
use tokio::sync::watch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command: Vec<String> = std::env::args().skip(1).collect();
    let config = config::TrustRiskConfig::from_env()?;
    if matches!(command.as_slice(), [action] if action == "migrate") {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&config.database_url)
            .await?;
        database::migrate(&pool).await?;
        println!("trust/risk database migrations applied");
        return Ok(());
    }
    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        database::connect_lazy(&config.database_url)?;
        println!("trust/risk runtime configuration is valid");
        return Ok(());
    }
    if let [action, kind, namespace, opaque_id, actor, reason] = command.as_slice()
        && action == "erase-subject"
    {
        let kind = kind
            .parse::<i16>()
            .map_err(|_| anyhow::anyhow!("subject kind must be an integer"))?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&config.database_url)
            .await?;
        let request_id = retention::erase_subject(
            &pool,
            kind,
            namespace,
            opaque_id,
            actor,
            reason,
            config.retention,
        )
        .await?;
        println!("subject erasure accepted: {request_id}");
        return Ok(());
    }
    if !command.is_empty() {
        anyhow::bail!(
            "usage: nvbes-trust-risk-service [migrate|validate-runtime|erase-subject <kind> <namespace> <opaque-id> <actor> <reason>]"
        );
    }

    nvbes_observability::install_safe_panic_hook();
    nvbes_observability::init_tracing_with_config(
        nvbes_observability::TracingConfig {
            environment: &config.environment,
            otlp_endpoint: None,
            otlp_authorization_header: None,
        },
        "nvbes-trust-risk-service",
    );
    let bind_addr = config.bind_addr;
    let db = database::connect_lazy(&config.database_url)?;
    let state = app::TrustRiskState::new(config, db);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    let signals =
        TrustRiskSignalServiceServer::new(ingress_grpc::SignalService::new(state.clone()))
            .max_encoding_message_size(256 * 1024)
            .max_decoding_message_size(256 * 1024);
    let assessments = TrustRiskAssessmentServiceServer::new(
        assessment_grpc::AssessmentService::new(state.clone()),
    )
    .max_encoding_message_size(256 * 1024)
    .max_decoding_message_size(256 * 1024);
    let labels = TrustRiskLabelServiceServer::new(labels_grpc::LabelService::new(state.clone()))
        .max_encoding_message_size(256 * 1024)
        .max_decoding_message_size(256 * 1024);
    let operations = TrustRiskOperationsServiceServer::new(
        operations_grpc::OperationsService::new(state.clone()),
    )
    .max_encoding_message_size(256 * 1024)
    .max_decoding_message_size(256 * 1024);
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<TrustRiskSignalServiceServer<ingress_grpc::SignalService>>()
        .await;
    health_reporter
        .set_serving::<TrustRiskAssessmentServiceServer<assessment_grpc::AssessmentService>>()
        .await;
    health_reporter
        .set_serving::<TrustRiskLabelServiceServer<labels_grpc::LabelService>>()
        .await;
    health_reporter
        .set_serving::<TrustRiskOperationsServiceServer<operations_grpc::OperationsService>>()
        .await;

    let http = health::router(state.clone()).merge(risk_metrics::router(state.clone()));
    let router = tonic::service::Routes::from(http)
        .add_service(health_service)
        .add_service(signals)
        .add_service(assessments)
        .add_service(labels)
        .add_service(operations)
        .into_axum_router();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let projection_task = tokio::spawn(projection::run(state.clone(), shutdown_rx.clone()));
    let retention_task = tokio::spawn(retention::run(state.db.clone(), shutdown_rx.clone()));

    tracing::info!(%bind_addr, environment = %state.config.environment, "starting independent trust/risk service");
    let server =
        axum::serve(listener, router).with_graceful_shutdown(server_shutdown(shutdown_rx.clone()));
    tokio::select! {
        result = server => result?,
        _ = process_shutdown_signal() => {},
    }
    let _ = shutdown_tx.send(true);
    projection_task.await?;
    retention_task.await?;
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
    tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
}

#[cfg(not(unix))]
async fn process_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
#[path = "trust_risk.acceptance.tests.rs"]
mod acceptance_tests;
