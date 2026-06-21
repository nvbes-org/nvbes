use axum::http::StatusCode;
pub use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;
use std::time::Duration;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: crate::database::Database,
    pub jwt: crate::domains::auth::jwt::JwtService,
    pub observability: nvbes_observability::metrics::HttpMetrics,
    pub product_analytics: nvbes_product_analytics::ProductAnalytics,
    pub email: std::sync::Arc<dyn nvbes_email::EmailSender>,
    pub dpop_nonce: Option<std::sync::Arc<nvbes_dpop::DpopNonceStore>>,
    pub redis: nvbes_redis::RedisPool,
    pub rate_limiter: nvbes_core::limiter::RateLimiter,
    pub allowed_browser_origins: crate::http::cors::AllowedOriginRegistry,
}

impl axum::extract::FromRef<AppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

impl AppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        if config.kms_enabled {
            anyhow::bail!(
                "NVBES_KMS_ENABLED requires a Cloud KMS adapter. OSS identity-api uses local JWT signing keys."
            );
        }

        let local_key =
            crate::domains::auth::keys_local::load_or_create_local_signing_key_material()
                .expect("local signing key material should load");
        crate::domains::auth::keys_local::ensure_local_signing_key(&db, &local_key)
            .await
            .expect("local signing key should be synced");
        let public_keys = crate::domains::auth::keys::get_public_key_pems_for_decoding(&db)
            .await
            .unwrap_or_default();

        info!(
            "JWT signing initialized with local RSA key (kid={})",
            local_key.kid
        );
        let jwt = crate::domains::auth::jwt::JwtService::new_local(
            &local_key.kid,
            &local_key.private_key_pem,
            public_keys,
            "nvbes-identity",
            "nvbes-identity-api",
            chrono::Duration::hours(config.auth_refresh_token_ttl_hours),
        );

        let redis = nvbes_core::redis_runtime::require_redis_pool(config).await?;

        let rate_limiter = nvbes_core::limiter::RateLimiter::new(redis.clone());
        let dpop_nonce = if config.dpop_enabled {
            Some(std::sync::Arc::new(nvbes_dpop::DpopNonceStore::new(
                redis.clone(),
                300,
            )))
        } else {
            None
        };

        let state = Self {
            config: config.clone(),
            db: db.clone(),
            jwt,
            observability: nvbes_observability::metrics::HttpMetrics::default(),
            product_analytics: build_product_analytics(config)?,
            email: build_email_sender(config)?,
            dpop_nonce,
            redis,
            rate_limiter,
            allowed_browser_origins: crate::http::cors::AllowedOriginRegistry::default(),
        };

        crate::database::ensure_default_oauth_clients_seeded(&state.db).await?;
        state
            .allowed_browser_origins
            .refresh_from_db(&state.db, &state.config)
            .await
            .map_err(|err| anyhow::anyhow!(err.message))?;

        state.observability.record_postgres_pool(
            &state.config.app_name,
            &state.config.environment,
            state.db.size(),
            state.db.num_idle(),
        );

        Ok(state)
    }
}

fn build_product_analytics(
    config: &AppConfig,
) -> anyhow::Result<nvbes_product_analytics::ProductAnalytics> {
    Ok(nvbes_product_analytics::ProductAnalytics::new(
        nvbes_product_analytics::ProductAnalyticsConfig {
            enabled: config.product_analytics_enabled,
            analytics_id_salt: config.analytics_id_salt.clone(),
        },
    )?)
}

fn build_email_sender(
    config: &AppConfig,
) -> anyhow::Result<std::sync::Arc<dyn nvbes_email::EmailSender>> {
    match config.email_provider.as_str() {
        "smtp" => {
            let host = config.smtp_host.clone().ok_or_else(|| {
                anyhow::anyhow!("NVBES_SMTP_HOST is required when NVBES_EMAIL_PROVIDER=smtp")
            })?;
            if config.environment != "development" && config.email_from_email.is_none() {
                anyhow::bail!(
                    "NVBES_EMAIL_FROM_EMAIL is required outside development when SMTP email is enabled"
                );
            }
            info!(
                "Email sender: SMTP (host={host}, port={})",
                config.smtp_port
            );
            Ok(std::sync::Arc::new(nvbes_email::SmtpEmailSender::new(
                nvbes_email::SmtpEmailConfig {
                    host,
                    port: config.smtp_port,
                    username: config.smtp_username.clone(),
                    password: config.smtp_password.clone(),
                    starttls: config.smtp_starttls,
                },
            )?))
        }
        "mock" => {
            if config.environment != "development" {
                anyhow::bail!(
                    "Mock email sender is forbidden outside development. Configure NVBES_EMAIL_PROVIDER=smtp."
                );
            }
            info!("Email sender: Mock (development mode)");
            Ok(std::sync::Arc::new(nvbes_email::MockEmailSender::new()))
        }
        provider => anyhow::bail!("Unsupported NVBES_EMAIL_PROVIDER={provider}"),
    }
}

pub fn build_router(state: AppState) -> axum::Router {
    crate::http::router(&state)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            nvbes_observability::middleware::observe_request,
        ))
        .layer(axum::middleware::from_fn(crate::http::security_headers))
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            nvbes_core::http::client_ip::trusted_client_ip_middleware,
        ))
        .layer(
            nvbes_core::http::connection_limit::PerIpConcurrencyLayer::new(
                state.config.api_max_connections_per_ip,
            ),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.http_request_timeout_secs),
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::http::cors::cors_middleware,
        ))
        .layer(ConcurrencyLimitLayer::new(
            state.config.api_max_concurrent_requests as usize,
        ))
        .with_state(state)
}
