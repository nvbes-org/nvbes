#[path = "identity.app.rs"]
mod app;
#[path = "identity.auth.rs"]
mod auth;
#[path = "identity.authz.rs"]
mod authz;
#[path = "identity.config.rs"]
mod config;
#[path = "identity.database.rs"]
mod database;
#[path = "identity.discovery.rs"]
mod discovery;
#[path = "identity.email.rs"]
mod email;
#[path = "identity.error_reporting.rs"]
mod error_reporting;
#[path = "identity.health.rs"]
mod health;
#[path = "identity.http.rs"]
mod http;
#[path = "identity.metrics.rs"]
mod metrics;
#[path = "identity.mfa.rs"]
mod mfa;
#[path = "identity.mfa.crypto.rs"]
mod mfa_crypto;
#[path = "identity.mfa.rotation.rs"]
mod mfa_rotation;
#[path = "identity.oauth.rs"]
mod oauth;
#[path = "identity.oauth_clients.rs"]
mod oauth_clients;
#[path = "identity.operator.rs"]
mod operator;
#[path = "identity.refresh.rs"]
mod refresh;
#[path = "identity.session.rs"]
mod session;
#[path = "identity.synthetic.rs"]
mod synthetic;
#[path = "identity.tokens.rs"]
mod tokens;
#[path = "identity.tokens.config.rs"]
mod tokens_config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(std::env::args().skip(1).collect()).await
}

async fn run(command: Vec<String>) -> anyhow::Result<()> {
    if matches!(command.as_slice(), [action] if action == "error-reporting-smoke") {
        let config = error_reporting::ErrorReportingRuntimeConfig::from_env()?;
        nvbes_observability::install_safe_panic_hook();
        let _error_reporting_guard = error_reporting::init(&config);
        let result = error_reporting::smoke(&config);
        println!("{}", serde_json::to_string(&result)?);
        anyhow::ensure!(
            result.configured && result.flushed && result.status == "sent",
            "Identity error-reporting smoke was not delivered"
        );
        return Ok(());
    }
    if matches!(command.as_slice(), [action] if action == "migrate") {
        let database_url = config::database_url_from_env()?;
        let pool = database::connect(&database_url, 2).await?;
        database::migrate(&pool).await?;
        println!("identity database migrations applied");
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-auth-smoke") {
        let database_url = config::database_url_from_env()?;
        let email = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let initial_password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let recovered_password = required_secret("NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD")?;
        let pool = database::connect(&database_url, 2).await?;
        let result = synthetic::run(&pool, &email, &initial_password, &recovered_password).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-auth-email-smoke") {
        let email_address = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let initial_password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let recovered_password = required_secret("NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD")?;
        let recovery_base_url = required_secret("NVBES_IDENTITY_RECOVERY_BASE_URL")?;
        let runtime = config::IdentityConfig::from_env()?;
        let email_config = nvbes_email::EmailClientConfig::from_env(&runtime.environment)?;
        let email_client = nvbes_email::EmailClient::connect(email_config).await?;
        let pool = database::connect(&runtime.database_url, 2).await?;
        let result = synthetic::run_with_delivery(
            &pool,
            &email_address,
            &initial_password,
            &recovered_password,
            |recovery| email::deliver_recovery(&email_client, &recovery_base_url, recovery),
        )
        .await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-mfa-smoke") {
        let email = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let runtime = config::IdentityConfig::from_env()?;
        let pool = database::connect(&runtime.database_url, 2).await?;
        let crypto = mfa_crypto::MfaCrypto::with_rotation(
            runtime.mfa_key_version,
            runtime.mfa_encryption_key,
            runtime
                .mfa_previous_key_version
                .zip(runtime.mfa_previous_encryption_key),
        )?;
        let result = mfa::run_synthetic_smoke(&pool, &crypto, &email, &password).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-token-smoke") {
        let email = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let audience = required_secret("NVBES_IDENTITY_SYNTHETIC_TOKEN_AUDIENCE")?;
        let runtime = config::IdentityConfig::from_env()?;
        let token_config = tokens_config::TokenConfig::from_env(&runtime.environment)?;
        let token_service = tokens::TokenService::new(token_config)?;
        let pool = database::connect(&runtime.database_url, 2).await?;
        let result =
            tokens::run_synthetic_smoke(&pool, &token_service, &email, &password, &audience)
                .await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "rotate-mfa-key") {
        let runtime = config::IdentityConfig::from_env()?;
        let crypto = mfa_crypto::MfaCrypto::with_rotation(
            runtime.mfa_key_version,
            runtime.mfa_encryption_key,
            runtime
                .mfa_previous_key_version
                .zip(runtime.mfa_previous_encryption_key),
        )?;
        let pool = database::connect(&runtime.database_url, 2).await?;
        let rotated = mfa_rotation::rotate(&pool, &crypto).await?;
        println!("{{\"rotated_factors\":{rotated}}}");
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        let config = config::IdentityConfig::from_env()?;
        database::connect_lazy(&config.database_url, config.database_max_connections)?;
        println!("identity runtime configuration is valid");
        return Ok(());
    }
    if !command.is_empty() {
        anyhow::bail!(
            "usage: nvbes-identity-service [migrate|validate-runtime|error-reporting-smoke|synthetic-auth-smoke|synthetic-auth-email-smoke|synthetic-mfa-smoke|synthetic-token-smoke|rotate-mfa-key]"
        );
    }

    let config = config::IdentityConfig::from_env()?;
    nvbes_observability::install_safe_panic_hook();
    let _error_reporting_guard = nvbes_observability::init_error_reporting_with_config(
        nvbes_observability::ErrorReportingConfig {
            app_name: error_reporting::APP_NAME,
            service_name: error_reporting::APP_NAME,
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
        "nvbes-identity-service",
    );
    let db = database::connect_lazy(&config.database_url, config.database_max_connections)?;
    let state = app::IdentityState::new(config.clone(), db.clone());
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    serve(state, db, listener, shutdown_signal()).await
}

async fn serve(
    state: app::IdentityState,
    db: sqlx::PgPool,
    listener: tokio::net::TcpListener,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let bind_addr = listener.local_addr()?;
    let http_metrics = nvbes_observability::metrics::HttpMetrics {
        handle: state.metrics.clone(),
    };
    let router = health::router(&state)
        .merge(metrics::router(&state))
        .merge(http::router(&state))
        .merge(oauth::router(&state))
        .merge(operator::router(&state))
        .merge(authz::router(&state))
        .merge(discovery::router(&state))
        .layer(axum::middleware::from_fn_with_state(
            http_metrics,
            nvbes_observability::middleware::observe_request,
        ));

    tracing::info!(%bind_addr, environment = %state.config.environment, "starting closed identity foundation");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    nvbes_observability::flush_error_reporting(std::time::Duration::from_secs(2));
    db.close().await;
    Ok(())
}

fn required_secret(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler must register");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
#[path = "identity.main.tests.rs"]
mod main_tests;
