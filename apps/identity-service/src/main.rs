#[path = "identity.app.rs"]
mod app;
#[path = "identity.auth.rs"]
mod auth;
#[path = "identity.config.rs"]
mod config;
#[path = "identity.database.rs"]
mod database;
#[path = "identity.email.rs"]
mod email;
#[path = "identity.error_reporting.rs"]
mod error_reporting;
#[path = "identity.health.rs"]
mod health;
#[path = "identity.invitations.rs"]
mod invitations;
#[path = "identity.metrics.rs"]
mod metrics;
#[path = "identity.mfa.rs"]
mod mfa;
#[path = "identity.mfa.crypto.rs"]
mod mfa_crypto;
#[path = "identity.mfa.rotation.rs"]
mod mfa_rotation;
#[path = "identity.recovery.rs"]
mod recovery;
#[path = "identity.recovery.commands.rs"]
mod recovery_commands;
#[path = "identity.recovery.delivery.rs"]
mod recovery_delivery;
#[path = "identity.recovery.http.rs"]
mod recovery_http;
#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.recovery.test-fixture.rs"]
mod recovery_test_fixture;
#[path = "identity.sessions.lock.rs"]
mod session_locks;
#[path = "identity.synthetic.rs"]
mod synthetic;
#[path = "identity.tokens.synthetic.rs"]
mod tokens_synthetic;
use nvbes_identity_service::{tokens, tokens_config};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command: Vec<String> = std::env::args().skip(1).collect();
    if recovery_commands::run(&command).await? {
        return Ok(());
    }
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
        let runtime = config::IdentityConfig::from_env()?;
        let email_address = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let initial_password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let recovered_password = required_secret("NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD")?;
        let recovery_base_url = required_secret("NVBES_IDENTITY_RECOVERY_BASE_URL")?;
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
        let runtime = config::mfa_runtime_config_from_env()?;
        let email = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let pool = database::connect(&runtime.database_url, 2).await?;
        let crypto = mfa_crypto::MfaCrypto::with_rotation(
            runtime.key_version,
            runtime.encryption_key,
            runtime
                .previous_key_version
                .zip(runtime.previous_encryption_key),
        )?;
        let result = mfa::run_synthetic_smoke(&pool, &crypto, &email, &password).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-invitation-smoke") {
        let database_url = config::database_url_from_env()?;
        let inviter_email = required_secret("NVBES_IDENTITY_SYNTHETIC_INVITER_EMAIL")?;
        let invited_email = required_secret("NVBES_IDENTITY_SYNTHETIC_INVITED_EMAIL")?;
        let password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let pool = database::connect(&database_url, 2).await?;
        let result =
            invitations::run_synthetic_smoke(&pool, &inviter_email, &invited_email, &password)
                .await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-token-smoke") {
        let environment = config::environment_from_env();
        let database_url = config::database_url_from_env()?;
        let email = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL")?;
        let password = required_secret("NVBES_IDENTITY_SYNTHETIC_PASSWORD")?;
        let audience = required_secret("NVBES_IDENTITY_SYNTHETIC_TOKEN_AUDIENCE")?;
        let token_config = tokens_config::TokenConfig::from_env(&environment)?;
        let token_service = tokens::TokenService::new(token_config)?;
        let pool = database::connect(&database_url, 2).await?;
        let result = tokens_synthetic::run_synthetic_smoke(
            &pool,
            &token_service,
            &email,
            &password,
            &audience,
        )
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

    if matches!(command.as_slice(), [action] if action == "dispatch-security-notifications") {
        let database_url = config::database_url_from_env()?;
        let email_config =
            nvbes_email::EmailClientConfig::from_env(&config::environment_from_env())?;
        let email_client = nvbes_email::EmailClient::connect(email_config).await?;
        let pool = database::connect(&database_url, 2).await?;
        let result =
            nvbes_identity_service::notification_dispatch::run_batch(&pool, &email_client).await?;
        println!("{}", serde_json::to_string(&result)?);
        pool.close().await;
        return Ok(());
    }

    let config = config::IdentityConfig::from_env()?;
    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        database::connect_lazy(&config.database_url, config.database_max_connections)?;
        println!("identity runtime configuration is valid");
        return Ok(());
    }
    if !command.is_empty() {
        anyhow::bail!(
            "usage: nvbes-identity-service [migrate|validate-runtime|error-reporting-smoke|synthetic-auth-smoke|synthetic-auth-email-smoke|synthetic-mfa-smoke|synthetic-invitation-smoke|synthetic-token-smoke|rotate-mfa-key|dispatch-security-notifications|request-password-recovery|dispatch-password-recovery]"
        );
    }

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
    let http_metrics = nvbes_observability::metrics::HttpMetrics {
        handle: state.metrics.clone(),
    };
    let mut router = health::router(state.clone())
        .merge(metrics::router(state))
        .layer(axum::middleware::from_fn_with_state(
            http_metrics,
            nvbes_observability::middleware::observe_request,
        ));
    match tokens_config::TokenConfig::from_env(&config.environment) {
        Ok(token_config) => {
            let token_service = tokens::TokenService::new(token_config)?;
            let issuer = token_service.issuer().to_owned();
            let token_service = Arc::new(token_service);
            router = router.merge(nvbes_identity_service::oauth::http::router_with_logout(
                &issuer,
                Arc::clone(&token_service),
                std::env::var("NVBES_IDENTITY_BROWSER_ORIGIN").is_ok()
                    && std::env::var("NVBES_IDENTITY_OAUTH_CLIENTS_JSON").is_ok(),
            ));
            if let Ok(registry_json) = std::env::var("NVBES_IDENTITY_OAUTH_CLIENTS_JSON") {
                let limiter = nvbes_identity_service::rate_limits::RateLimiter::from_base64(
                    &required_secret("NVBES_IDENTITY_RATE_LIMIT_KEY")?,
                )?;
                let clients = nvbes_identity_service::oauth::clients::ClientRegistry::from_json(
                    &registry_json,
                    config.environment == "development" || config.environment == "test",
                )
                .map_err(|_| anyhow::anyhow!("invalid OIDC client registry"))?;
                let clients = Arc::new(clients);
                if let Ok(resources_json) = std::env::var("NVBES_IDENTITY_RESOURCE_SERVERS_JSON") {
                    let resources =
                        nvbes_identity_service::oauth::resources::ResourceServers::from_json(
                            &resources_json,
                        )?;
                    router =
                        router.merge(nvbes_identity_service::oauth::http::introspection_router(
                            db.clone(),
                            clients.clone(),
                            token_service.clone(),
                            Arc::new(resources),
                            limiter.clone(),
                        ));
                }
                router = router.merge(nvbes_identity_service::oauth::http::token_router(
                    db.clone(),
                    clients,
                    Arc::clone(&token_service),
                    limiter.clone(),
                ));
                if let Ok(origin) = std::env::var("NVBES_IDENTITY_BROWSER_ORIGIN") {
                    let browser = nvbes_identity_service::browser::BrowserSecurity::new(
                        &origin,
                        config.environment == "development" || config.environment == "test",
                    )
                    .map_err(|_| anyhow::anyhow!("invalid Identity browser origin"))?;
                    if std::env::var("NVBES_IDENTITY_PASSWORD_RECOVERY_ENABLED").as_deref()
                        == Ok("1")
                    {
                        anyhow::ensure!(
                            origin.starts_with("https://"),
                            "password recovery requires HTTPS"
                        );
                        let crypto = mfa_crypto::MfaCrypto::with_rotation(
                            config.mfa_key_version,
                            config.mfa_encryption_key,
                            config
                                .mfa_previous_key_version
                                .zip(config.mfa_previous_encryption_key),
                        )?;
                        router = router.merge(recovery_http::router(
                            db.clone(),
                            browser.clone(),
                            limiter.clone(),
                            Arc::new(crypto),
                            format!("{}/password-recovery", origin.trim_end_matches('/')),
                        ));
                    }
                    let mfa = nvbes_identity_service::mfa_crypto::MfaCrypto::with_rotation(
                        config.mfa_key_version,
                        config.mfa_encryption_key,
                        config
                            .mfa_previous_key_version
                            .zip(config.mfa_previous_encryption_key),
                    )?;
                    let clients =
                        nvbes_identity_service::oauth::clients::ClientRegistry::from_json(
                            &registry_json,
                            config.environment == "development" || config.environment == "test",
                        )
                        .map_err(|_| anyhow::anyhow!("invalid OIDC client registry"))?;
                    let clients = Arc::new(clients);
                    router = router.merge(nvbes_identity_service::oauth::http::rp_logout_router(
                        db.clone(),
                        clients.clone(),
                        token_service.clone(),
                        browser.clone(),
                        limiter.clone(),
                    ));
                    router =
                        router.merge(nvbes_identity_service::oauth::http::authorization_router(
                            db.clone(),
                            clients.clone(),
                            browser.clone(),
                            Arc::new(mfa),
                            limiter.clone(),
                        ));
                    let rp_origin = reqwest::Url::parse(&origin)?;
                    let rp_id = rp_origin
                        .host_str()
                        .ok_or_else(|| anyhow::anyhow!("missing WebAuthn RP hostname"))?;
                    let webauthn = nvbes_identity_service::webauthn::build_server(rp_id, &origin)
                        .map_err(|error| anyhow::anyhow!(error))?;
                    let webauthn = Arc::new(webauthn);
                    router =
                        router.merge(nvbes_identity_service::oauth::http::passkey_login_router(
                            db.clone(),
                            clients,
                            browser.clone(),
                            webauthn.clone(),
                            limiter.clone(),
                        ));
                    router = router.merge(nvbes_identity_service::oauth::http::recovery_router(
                        db.clone(),
                        browser.clone(),
                        webauthn.clone(),
                        limiter.clone(),
                    ));
                    router = router.merge(nvbes_identity_service::oauth::http::webauthn_router(
                        db.clone(),
                        browser,
                        webauthn,
                        limiter,
                    ));
                    tracing::info!("OAuth authorization interaction endpoint enabled");
                }
                tracing::info!("OAuth authorization-code token endpoint enabled");
            } else {
                tracing::warn!("OAuth token endpoint remains disabled: client registry is absent");
            }
            tracing::info!("OIDC discovery and JWKS endpoints enabled");
        }
        Err(error) if config.environment == "development" || config.environment == "test" => {
            tracing::warn!(%error, "OIDC discovery and JWKS endpoints remain disabled: token configuration is absent");
        }
        Err(error) => return Err(error.into()),
    }
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(bind_addr = %config.bind_addr, environment = %config.environment, "starting closed identity foundation");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
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
