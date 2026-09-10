#[path = "account.app.rs"]
mod app;
#[path = "account.audit.rs"]
mod audit;
#[path = "account.auth.rs"]
mod auth;
#[path = "account.billing_authorization.rs"]
mod billing_authorization;
#[path = "account.config.rs"]
mod config;
#[path = "account.database.rs"]
mod database;
#[path = "account.error.rs"]
mod error;
#[path = "account.health.rs"]
mod health;
#[path = "account.metrics.rs"]
mod metrics;
#[path = "account.preferences.rs"]
mod preferences;
#[path = "account.privacy.rs"]
mod privacy;
#[path = "account.privacy.jobs.rs"]
mod privacy_jobs;
#[path = "account.profile.rs"]
mod profile;
#[path = "account.synthetic.rs"]
mod synthetic;
#[path = "account.teams.rs"]
mod teams;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command: Vec<String> = std::env::args().skip(1).collect();
    if matches!(command.as_slice(), [action] if action == "migrate") {
        let config = config::AccountConfig::from_env()?;
        let pool = database::connect(&config.database_url, 2).await?;
        database::migrate(&pool).await?;
        println!("account database migrations applied");
        return Ok(());
    }
    if matches!(command.as_slice(), [action] if action == "synthetic-account-smoke") {
        let config = config::AccountConfig::from_env()?;
        let owner = required_uuid("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID")?;
        let member = required_uuid("NVBES_ACCOUNT_SYNTHETIC_MEMBER_ID")?;
        let pool = database::connect(&config.database_url, 2).await?;
        let result = synthetic::run(&pool, owner, member).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if matches!(command.as_slice(), [action] if action == "process-privacy-jobs") {
        let config = config::AccountConfig::from_env()?;
        let pool = database::connect(&config.database_url, 2).await?;
        let result = privacy_jobs::process_pending(&pool).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let config = config::AccountConfig::from_env()?;
    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        database::connect_lazy(&config.database_url, config.database_max_connections)?;
        auth::TokenVerifier::new(&config)?;
        println!("account runtime configuration is valid");
        return Ok(());
    }
    if !command.is_empty() {
        anyhow::bail!(
            "usage: nvbes-account-service [migrate|validate-runtime|synthetic-account-smoke|process-privacy-jobs]"
        );
    }

    nvbes_observability::install_safe_panic_hook();
    let _error_guard = nvbes_observability::init_error_reporting_with_config(
        nvbes_observability::ErrorReportingConfig {
            app_name: "nvbes-account-service",
            service_name: "nvbes-account-service",
            environment: &config.environment,
            dsn: config.sentry_dsn.as_deref(),
            traces_sample_rate: config.sentry_traces_sample_rate,
        },
    );
    nvbes_observability::init_tracing_with_config(
        nvbes_observability::TracingConfig {
            environment: &config.environment,
            otlp_endpoint: config.otlp_endpoint.as_deref(),
            otlp_authorization_header: config.otlp_authorization_header.as_deref(),
            protocol: nvbes_observability::OtlpProtocol::Http,
        },
        "nvbes-account-service",
    );
    let db = database::connect_lazy(&config.database_url, config.database_max_connections)?;
    let state = app::AccountState::new(config.clone(), db.clone())?;
    let http_metrics = nvbes_observability::metrics::HttpMetrics {
        handle: state.metrics.clone(),
    };
    let router = app::router(state).layer(axum::middleware::from_fn_with_state(
        http_metrics,
        nvbes_observability::middleware::observe_request,
    ));
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(bind_addr = %config.bind_addr, environment = %config.environment, "starting Account runtime");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    nvbes_observability::flush_error_reporting(std::time::Duration::from_secs(2));
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
