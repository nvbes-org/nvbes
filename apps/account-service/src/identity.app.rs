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
    pub otp_provider: std::sync::Arc<dyn crate::domains::auth::otp_provider::OtpProvider>,
    pub dpop_nonce: Option<std::sync::Arc<nvbes_dpop::DpopNonceStore>>,
    pub redis: nvbes_redis::RedisPool,
    pub rate_limiter: nvbes_core::limiter::RateLimiter,
    pub allowed_browser_origins: crate::http::cors::AllowedOriginRegistry,
    pub storage: std::sync::Arc<dyn nvbes_storage::ObjectStore>,
}

impl axum::extract::FromRef<AppState> for nvbes_observability::metrics::HttpMetrics {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

impl AppState {
    pub async fn bootstrap(config: &AppConfig, db: PgPool) -> anyhow::Result<Self> {
        let jwt = if config.kms_enabled {
            let signer = crate::domains::auth::jwt::kms::ScalewayKmsSigner::from_env()
                .map_err(|error| anyhow::anyhow!(error.message))?;
            let public_key_pem = signer
                .public_key_pem()
                .await
                .map_err(|error| anyhow::anyhow!(error.message))?;
            let kid = crate::domains::auth::keys::key_id_from_public_key(&public_key_pem)
                .map_err(|error| anyhow::anyhow!(error.message))?;
            crate::domains::auth::keys::activate_kms_key(
                &db,
                &kid,
                signer.key_id(),
                &public_key_pem,
            )
            .await
            .map_err(|error| anyhow::anyhow!(error.message))?;
            let public_keys = crate::domains::auth::keys::get_public_key_pems_for_decoding(&db)
                .await
                .map_err(|error| anyhow::anyhow!(error.message))?;
            info!(%kid, kms_key_id = signer.key_id(), "JWT signing initialized with Scaleway KMS");
            crate::domains::auth::jwt::JwtService::new_kms(
                &kid,
                signer,
                public_keys,
                config.api_base_url.trim_end_matches('/'),
                "nvbes-account-service",
                chrono::Duration::hours(config.auth_refresh_token_ttl_hours),
            )
        } else {
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
                config.api_base_url.trim_end_matches('/'),
                "nvbes-account-service",
                chrono::Duration::hours(config.auth_refresh_token_ttl_hours),
            )
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
            email: nvbes_product_account::email::delivery::build_email_sender(config)?,
            otp_provider: build_otp_provider(config)?,
            dpop_nonce,
            redis,
            rate_limiter,
            allowed_browser_origins: crate::http::cors::AllowedOriginRegistry::default(),
            storage: nvbes_product_cloud::storage::build_storage(config).await,
        };

        crate::database::ensure_default_oauth_clients_seeded(&state.db).await?;
        state
            .allowed_browser_origins
            .refresh_from_db(&state.db, &state.config)
            .await
            .map_err(|err| anyhow::anyhow!(err.message))?;

        state.observability.record_postgres_pool(
            "account-service",
            &state.config.environment,
            state.db.size(),
            state.db.num_idle(),
        );

        Ok(state)
    }
}

fn build_otp_provider(
    config: &AppConfig,
) -> anyhow::Result<std::sync::Arc<dyn crate::domains::auth::otp_provider::OtpProvider>> {
    match config.otp_provider.as_str() {
        "twilio_verify" => {
            let account_sid = config.twilio_account_sid.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "NVBES_TWILIO_ACCOUNT_SID is required when NVBES_OTP_PROVIDER=twilio_verify"
                )
            })?;
            let auth_token = config.twilio_auth_token.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "NVBES_TWILIO_AUTH_TOKEN is required when NVBES_OTP_PROVIDER=twilio_verify"
                )
            })?;
            let service_sid = config.twilio_verify_service_sid.clone().ok_or_else(|| {
                anyhow::anyhow!("NVBES_TWILIO_VERIFY_SERVICE_SID is required when NVBES_OTP_PROVIDER=twilio_verify")
            })?;
            info!("OTP provider: Twilio Verify");
            Ok(std::sync::Arc::new(
                crate::domains::auth::otp_twilio::TwilioVerifyOtpProvider::new(
                    account_sid,
                    auth_token,
                    service_sid,
                    config.twilio_api_base_url.clone(),
                ),
            ))
        }
        "mock" => {
            if config.environment != "development" {
                anyhow::bail!("Mock OTP provider is forbidden outside development.");
            }
            info!("OTP provider: Mock (development mode)");
            Ok(std::sync::Arc::new(
                crate::domains::auth::otp_mock::MockOtpProvider::new(),
            ))
        }
        provider => anyhow::bail!("Unsupported NVBES_OTP_PROVIDER={provider}"),
    }
}

fn build_product_analytics(
    config: &AppConfig,
) -> anyhow::Result<nvbes_product_analytics::ProductAnalytics> {
    let analytics_config = nvbes_product_analytics::ProductAnalyticsConfig {
        enabled: config.product_analytics_enabled,
        analytics_id_salt: config.analytics_id_salt.clone(),
    };

    if !config.product_analytics_enabled {
        info!("Account product analytics disabled");
        return Ok(nvbes_product_analytics::ProductAnalytics::disabled());
    }

    let sink = nvbes_analytics_posthog::PostHogAnalyticsSink::new(
        nvbes_analytics_posthog::PostHogAnalyticsConfig {
            host: config.posthog_host.clone(),
            project_token: config.product_analytics_token.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "NVBES_PRODUCT_ANALYTICS_TOKEN is required when product analytics is enabled"
                )
            })?,
        },
    )?;
    info!("Account product analytics enabled with PostHog");

    Ok(nvbes_product_analytics::ProductAnalytics::with_sink(
        analytics_config,
        std::sync::Arc::new(sink),
    )?)
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
