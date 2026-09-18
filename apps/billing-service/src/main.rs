#[path = "billing.app.rs"]
mod app;
#[path = "billing.audit.rs"]
mod audit;
#[path = "billing.auth.rs"]
mod auth;
#[path = "billing.checkout.rs"]
mod checkout;
#[path = "billing.config.rs"]
mod config;
#[path = "billing.customer.rs"]
mod customer;
#[path = "billing.database.rs"]
mod database;
#[path = "billing.email.rs"]
mod email;
#[path = "billing.error.rs"]
mod error;
#[path = "billing.grpc.rs"]
mod grpc;
#[path = "billing.health.rs"]
mod health;
#[path = "billing.metrics.rs"]
mod metrics;
#[path = "billing.outbox.rs"]
mod outbox;
#[path = "billing.plans.rs"]
mod plans;
#[path = "billing.portal.rs"]
mod portal;
#[path = "billing.reconciliation.rs"]
mod reconciliation;
#[path = "billing.subscriptions.rs"]
mod subscriptions;
#[path = "billing.synthetic.rs"]
mod synthetic;
#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.webhooks.tests.rs"]
mod webhook_tests;
#[path = "billing.webhooks.rs"]
mod webhooks;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command: Vec<String> = std::env::args().skip(1).collect();

    if matches!(command.as_slice(), [action] if action == "migrate") {
        let config = config::BillingConfig::from_env()?;
        let pool = database::connect(&config.database_url, 2).await?;
        database::migrate(&pool).await?;
        println!("billing database migrations applied");
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-billing-smoke") {
        let config = config::BillingConfig::from_env()?;
        let workspace_id = required_uuid("NVBES_BILLING_SYNTHETIC_WORKSPACE_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4());
        let owner_id = required_uuid("NVBES_BILLING_SYNTHETIC_OWNER_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4());
        let pool = database::connect(&config.database_url, 2).await?;
        let result = synthetic::run(&pool, &config, workspace_id, owner_id).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "publish-outbox") {
        let config = config::BillingConfig::from_env()?;
        let pool = database::connect(&config.database_url, 2).await?;
        let email_client = if let (Some(endpoint), Some(token)) =
            (&config.email_grpc_endpoint, &config.email_token)
        {
            let email_cfg = nvbes_email::EmailClientConfig::from_values(
                "development",
                endpoint.clone(),
                token.clone(),
                std::time::Duration::from_secs(5),
            )?;
            Some(nvbes_email::EmailClient::connect(email_cfg).await?)
        } else {
            None
        };
        let published = outbox::publish_pending_outbox_events(
            &pool,
            email_client.as_ref(),
            &config.app_url,
            100,
        )
        .await?;
        println!("published {published} outbox events");
        return Ok(());
    }

    let config = config::BillingConfig::from_env()?;
    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        database::connect_lazy(&config.database_url, 2)?;
        auth::TokenVerifier::new(&config)?;
        println!("billing runtime configuration is valid");
        return Ok(());
    }

    if !command.is_empty() && command[0] != "serve" {
        anyhow::bail!(
            "usage: nvbes-billing-service [serve|migrate|validate-runtime|publish-outbox|synthetic-billing-smoke]"
        );
    }

    let db = database::connect(&config.database_url, 5).await?;
    let prometheus_handle = metrics::install();
    let token_verifier = auth::TokenVerifier::new(&config)?;

    let email_client =
        if let (Some(endpoint), Some(token)) = (&config.email_grpc_endpoint, &config.email_token) {
            if let Ok(email_cfg) = nvbes_email::EmailClientConfig::from_values(
                "development",
                endpoint.clone(),
                token.clone(),
                std::time::Duration::from_secs(5),
            ) {
                nvbes_email::EmailClient::connect(email_cfg)
                    .await
                    .ok()
                    .map(std::sync::Arc::new)
            } else {
                None
            }
        } else {
            None
        };

    let state = app::BillingState {
        db: db.clone(),
        config: config.clone(),
        metrics: prometheus_handle,
        tokens: token_verifier,
        email_client: email_client.clone(),
    };

    let outbox_db = db.clone();
    let outbox_email = email_client.clone();
    let outbox_app_url = config.app_url.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let _ = outbox::publish_pending_outbox_events(
                &outbox_db,
                outbox_email.as_deref(),
                &outbox_app_url,
                50,
            )
            .await;
        }
    });

    let delivery_grpc = grpc::delivery_server(grpc::BillingDeliveryGrpcService::new(state.clone()));
    let operations_grpc =
        grpc::operations_server(grpc::BillingOperationsGrpcService::new(state.clone()));

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_service_status(
            "nvbes.billing.v1.BillingDeliveryService",
            tonic_health::ServingStatus::Serving,
        )
        .await;
    health_reporter
        .set_service_status(
            "nvbes.billing.v1.BillingOperationsService",
            tonic_health::ServingStatus::Serving,
        )
        .await;

    let router = app::create_router(state);
    let combined = tonic::service::Routes::from(router)
        .add_service(health_service)
        .add_service(delivery_grpc)
        .add_service(operations_grpc)
        .into_axum_router();

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(bind_addr = %config.bind_addr, "starting Billing runtime (Stripe test mode only, gRPC+HTTP)");

    axum::serve(listener, combined)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    db.close().await;
    Ok(())
}

fn required_uuid(name: &str) -> anyhow::Result<uuid::Uuid> {
    let value = std::env::var(name).map_err(|_| anyhow::anyhow!("{name} is required"))?;
    Ok(uuid::Uuid::parse_str(&value)?)
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler must register");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
