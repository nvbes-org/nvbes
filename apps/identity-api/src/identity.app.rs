use axum::http::StatusCode;
pub use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;
use std::time::Duration;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

use crate::domains::auth::keys::KeyBackend;

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
}

impl axum::extract::FromRef<AppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

impl AppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        let backend = if config.kms_enabled {
            let kms_config = nvbes_core::scw_kms::KmsConfig::from_env().unwrap_or_else(|| {
                panic!(
                    "KMS enabled but SCW_ACCESS_KEY, SCW_SECRET_KEY, and SCW_DEFAULT_PROJECT_ID must be set"
                )
            });
            let client = nvbes_core::scw_kms::KmsClient::new(kms_config);
            KeyBackend::Kms(client)
        } else {
            if config.environment != "development" {
                panic!(
                    "Local JWT signing keys are forbidden outside development. Enable KMS before starting this environment."
                );
            }
            KeyBackend::Local
        };

        let jwt = match &backend {
            KeyBackend::Kms(_) => {
                crate::domains::auth::keys::create_initial_key(&db, &backend)
                    .await
                    .expect("initial signing key creation should succeed");

                let active_key = crate::domains::auth::keys::get_active_key(&db)
                    .await
                    .expect("active key lookup should succeed")
                    .expect("active key should exist after create_initial_key");
                let public_keys = crate::domains::auth::keys::get_public_key_pems_for_decoding(&db)
                    .await
                    .unwrap_or_default();

                info!(
                    "JWT signing initialized with KMS (kid={}, kms_key_id={})",
                    active_key.kid,
                    active_key.kms_key_id.as_deref().unwrap_or("unknown")
                );
                crate::domains::auth::jwt::JwtService::new_kms(
                    &active_key,
                    backend,
                    public_keys,
                    "nvbes-identity",
                    "nvbes-identity-api",
                    chrono::Duration::hours(config.auth_refresh_token_ttl_hours),
                )
            }
            KeyBackend::Local => {
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
                crate::domains::auth::jwt::JwtService::new_local(
                    &local_key.kid,
                    &local_key.private_key_pem,
                    public_keys,
                    "nvbes-identity",
                    "nvbes-identity-api",
                    chrono::Duration::hours(config.auth_refresh_token_ttl_hours),
                )
            }
        };

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
            email: build_email_sender(config),
            dpop_nonce,
            redis,
            rate_limiter,
        };

        crate::database::ensure_default_oauth_clients_seeded(&state.db).await?;

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
            enabled: config.posthog_enabled,
            host: config.posthog_host.clone(),
            project_token: config.posthog_project_token.clone(),
            analytics_id_salt: config.analytics_id_salt.clone(),
        },
    )?)
}

fn build_email_sender(config: &AppConfig) -> std::sync::Arc<dyn nvbes_email::EmailSender> {
    if config.scw_tem_enabled {
        let secret_key = config
            .scw_secret_key
            .clone()
            .expect("SCW_SECRET_KEY required when SCW_TEM_ENABLED is true");
        let project_id = config
            .scw_project_id
            .clone()
            .expect("SCW_PROJECT_ID required when SCW_TEM_ENABLED is true");

        if config.environment != "development" {
            config
                .scw_tem_from_email
                .clone()
                .expect("SCW_TEM_FROM_EMAIL is required in non-development environments when SCW_TEM_ENABLED is true. Set it to a real monitored address for deliverability.");
        }

        info!(
            "Email sender: Scaleway Transactional Email (region={})",
            config.scw_region
        );
        std::sync::Arc::new(nvbes_email::ScalewayEmailClient::new(
            secret_key,
            project_id,
            &config.scw_region,
        ))
    } else {
        if config.environment != "development" {
            panic!(
                "Mock email sender is forbidden outside development. Configure SCW_TEM_ENABLED and production sender credentials."
            );
        }
        info!("Email sender: Mock (development mode)");
        std::sync::Arc::new(nvbes_email::MockEmailSender::new())
    }
}

pub fn build_router(state: AppState) -> axum::Router {
    let cors = nvbes_core::security::cors_layer(&state.config);

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
        .layer(cors)
        .layer(ConcurrencyLimitLayer::new(
            state.config.api_max_concurrent_requests as usize,
        ))
        .layer(sentry_tower::SentryHttpLayer::new().enable_transaction())
        .layer(sentry_tower::NewSentryLayer::new_from_top())
        .with_state(state)
}
