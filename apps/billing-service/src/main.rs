#[path = "billing.app.rs"]
mod app;
#[path = "billing.audit.rs"]
mod audit;
#[path = "billing.auth.rs"]
mod auth;
#[path = "billing.authorization.rs"]
mod authorization;
#[path = "billing.checkout.rs"]
mod checkout;
#[path = "billing.config.rs"]
mod config;
#[path = "billing.customer.rs"]
mod customer;
#[path = "billing.database.rs"]
mod database;
#[path = "billing.error.rs"]
mod error;
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

    let config = config::BillingConfig::from_env()?;
    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        database::connect_lazy(&config.database_url, 2)?;
        auth::TokenVerifier::new(&config)?;
        println!("billing runtime configuration is valid");
        return Ok(());
    }

    if !command.is_empty() && command[0] != "serve" {
        anyhow::bail!(
            "usage: nvbes-billing-service [serve|migrate|validate-runtime|synthetic-billing-smoke]"
        );
    }

    let db = database::connect(&config.database_url, 5).await?;
    let prometheus_handle = metrics::install();
    let token_verifier = auth::TokenVerifier::new(&config)?;

    let state = app::BillingState {
        db: db.clone(),
        config: config.clone(),
        metrics: prometheus_handle,
        tokens: token_verifier,
    };

    let router = app::create_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(bind_addr = %config.bind_addr, "starting Billing runtime (Stripe test mode only)");

    axum::serve(listener, router)
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
