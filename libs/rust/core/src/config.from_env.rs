use super::AppConfig;
use super::billing::billing_provider_env;
use super::env::{
    env_bool, env_bool_any, env_or_default, optional_env, optional_env_any, parse_csv,
};
use super::geo::ip_intelligence_env;
use super::validation::validate_config_urls_and_secrets;

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let environment = env_or_default(
            "NVBES_ENV",
            std::env::var("NVBES_ENV").ok(),
            "development",
            false,
        )?;
        let strict_mode = environment != "development";
        let secret_manager_enabled = env_bool("NVBES_SECRET_MANAGER_ENABLED", false);
        let secret_bootstrap_mode = strict_mode && !secret_manager_enabled;

        let api_port = std::env::var("NVBES_API_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(4000);

        let billing_provider_env = billing_provider_env()?;
        let ip_intelligence_env = ip_intelligence_env();

        let config = Self {
            app_name: env_or_default(
                "NVBES_APP_NAME",
                std::env::var("NVBES_APP_NAME").ok(),
                "nvbes Drive",
                false,
            )?,
            environment,
            api_port,
            web_base_url: env_or_default(
                "NVBES_WEB_BASE_URL",
                std::env::var("NVBES_WEB_BASE_URL").ok(),
                "http://localhost:5173",
                strict_mode,
            )?,
            api_base_url: env_or_default(
                "NVBES_API_BASE_URL",
                std::env::var("NVBES_API_BASE_URL").ok(),
                "http://localhost:4000",
                strict_mode,
            )?,
            staging_web_base_url: optional_env("NVBES_STAGING_WEB_BASE_URL"),
            staging_api_base_url: optional_env("NVBES_STAGING_API_BASE_URL"),
            additional_cors_origins: parse_csv(
                &std::env::var("NVBES_ADDITIONAL_CORS_ORIGINS").unwrap_or_default(),
            ),
            database_url: env_or_default(
                "NVBES_DATABASE_URL",
                std::env::var("NVBES_DATABASE_URL").ok(),
                "postgres://postgres:postgres@localhost:5432/nvbes",
                secret_bootstrap_mode,
            )?,
            billing_database_url: env_or_default(
                "NVBES_BILLING_DATABASE_URL",
                std::env::var("NVBES_BILLING_DATABASE_URL").ok(),
                "postgres://postgres:postgres@localhost:5432/nvbes_billing",
                secret_bootstrap_mode,
            )?,
            database_max_connections: std::env::var("NVBES_DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10),
            auth_session_ttl_hours: std::env::var("NVBES_AUTH_SESSION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(24 * 30),
            auth_session_idle_ttl_minutes: std::env::var("NVBES_AUTH_SESSION_IDLE_TTL_MINUTES")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(12 * 60),
            auth_refresh_token_ttl_hours: std::env::var("NVBES_AUTH_REFRESH_TOKEN_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(24 * 30),
            auth_verification_ttl_hours: std::env::var("NVBES_AUTH_VERIFICATION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(24),
            auth_verification_resend_cooldown_seconds: std::env::var(
                "NVBES_AUTH_VERIFICATION_RESEND_COOLDOWN_SECONDS",
            )
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(45),
            auth_unverified_account_ttl_days: std::env::var(
                "NVBES_AUTH_UNVERIFIED_ACCOUNT_TTL_DAYS",
            )
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(7),
            auth_password_reset_ttl_minutes: std::env::var("NVBES_AUTH_PASSWORD_RESET_TTL_MINUTES")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(30),
            auth_password_history_size: std::env::var("NVBES_AUTH_PASSWORD_HISTORY_SIZE")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(5),
            auth_password_pepper: optional_env("NVBES_AUTH_PASSWORD_PEPPER"),
            auth_factor_encryption_key: optional_env("NVBES_AUTH_FACTOR_ENCRYPTION_KEY"),
            auth_factor_encryption_key_version: std::env::var(
                "NVBES_AUTH_FACTOR_ENCRYPTION_KEY_VERSION",
            )
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1),
            auth_pow_enabled: std::env::var("NVBES_AUTH_POW_ENABLED")
                .ok()
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(true),
            auth_pow_difficulty: std::env::var("NVBES_AUTH_POW_DIFFICULTY")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(18),
            auth_pow_ttl_seconds: std::env::var("NVBES_AUTH_POW_TTL_SECONDS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(120),
            auth_step_up_ttl_minutes: std::env::var("NVBES_AUTH_STEP_UP_TTL_MINUTES")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(15),
            auth_device_trust_ttl_days: std::env::var("NVBES_AUTH_DEVICE_TRUST_TTL_DAYS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(90),
            stripe_secret_key: billing_provider_env.stripe_secret_key,
            stripe_webhook_secret: billing_provider_env.stripe_webhook_secret,
            stripe_api_base_url: billing_provider_env.stripe_api_base_url,
            mollie_api_key: billing_provider_env.mollie_api_key,
            mollie_api_base_url: billing_provider_env.mollie_api_base_url,
            billing_mollie_enabled: billing_provider_env.billing_mollie_enabled,
            billing_mollie_routing_status: billing_provider_env.billing_mollie_routing_status,
            billing_external_provider_fallback_enabled: billing_provider_env
                .billing_external_provider_fallback_enabled,
            billing_external_provider_routing_status: billing_provider_env
                .billing_external_provider_routing_status,
            billing_fraud_enforcement_enabled: billing_provider_env
                .billing_fraud_enforcement_enabled,
            billing_fraud_step_up_threshold: billing_provider_env.billing_fraud_step_up_threshold,
            billing_fraud_manual_review_threshold: billing_provider_env
                .billing_fraud_manual_review_threshold,
            billing_fraud_block_threshold: billing_provider_env.billing_fraud_block_threshold,
            billing_fraud_policy_overrides_json: billing_provider_env
                .billing_fraud_policy_overrides_json,
            billing_default_success_url: env_or_default(
                "NVBES_BILLING_SUCCESS_URL",
                std::env::var("NVBES_BILLING_SUCCESS_URL").ok(),
                "http://localhost:5173/billing/success",
                strict_mode,
            )?,
            billing_default_cancel_url: env_or_default(
                "NVBES_BILLING_CANCEL_URL",
                std::env::var("NVBES_BILLING_CANCEL_URL").ok(),
                "http://localhost:5173/billing/cancel",
                strict_mode,
            )?,
            billing_default_portal_return_url: env_or_default(
                "NVBES_BILLING_PORTAL_RETURN_URL",
                std::env::var("NVBES_BILLING_PORTAL_RETURN_URL").ok(),
                "http://localhost:5173/billing",
                strict_mode,
            )?,
            jwt_secret: env_or_default(
                "NVBES_JWT_SECRET",
                std::env::var("NVBES_JWT_SECRET").ok(),
                "default-secret-change-me",
                secret_bootstrap_mode,
            )?,
            webauthn_rp_id: env_or_default(
                "NVBES_WEBAUTHN_RP_ID",
                std::env::var("NVBES_WEBAUTHN_RP_ID").ok(),
                "localhost",
                strict_mode,
            )?,
            webauthn_rp_origin: env_or_default(
                "NVBES_WEBAUTHN_RP_ORIGIN",
                std::env::var("NVBES_WEBAUTHN_RP_ORIGIN").ok(),
                "http://localhost:3001",
                strict_mode,
            )?,
            webauthn_related_origins: optional_env("NVBES_WEBAUTHN_RELATED_ORIGINS")
                .map(|v| parse_csv(&v))
                .unwrap_or_default(),
            sentry_dsn: optional_env_any(&["SENTRY_DSN", "NVBES_SENTRY_DSN"]),
            sentry_traces_sample_rate: std::env::var("SENTRY_TRACES_SAMPLE_RATE")
                .or_else(|_| std::env::var("NVBES_SENTRY_TRACES_SAMPLE_RATE"))
                .ok()
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0),
            otlp_endpoint: optional_env("NVBES_OTLP_ENDPOINT"),
            otlp_authorization_header: optional_env("NVBES_OTLP_AUTHORIZATION_HEADER"),
            product_analytics_enabled: env_bool_any(
                &["NVBES_POSTHOG_ENABLED", "NVBES_PRODUCT_ANALYTICS_ENABLED"],
                false,
            ),
            product_analytics_token: optional_env_any(&[
                "NVBES_POSTHOG_PROJECT_TOKEN",
                "NVBES_PRODUCT_ANALYTICS_TOKEN",
            ]),
            posthog_host: env_or_default(
                "NVBES_POSTHOG_HOST",
                optional_env("NVBES_POSTHOG_HOST"),
                "https://eu.i.posthog.com",
                false,
            )?,
            analytics_id_salt: optional_env("NVBES_ANALYTICS_ID_SALT"),
            profiling_enabled: env_bool("NVBES_PROFILING_ENABLED", false),
            profiling_endpoint: optional_env("NVBES_PROFILING_ENDPOINT"),
            profiling_sample_rate_hz: std::env::var("NVBES_PROFILING_SAMPLE_RATE_HZ")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(100),
            profiling_basic_auth_user: optional_env("NVBES_PROFILING_BASIC_AUTH_USER"),
            profiling_basic_auth_password: optional_env("NVBES_PROFILING_BASIC_AUTH_PASSWORD"),
            observability_internal_token: optional_env("NVBES_OBSERVABILITY_INTERNAL_TOKEN"),
            kms_enabled: std::env::var("NVBES_KMS_ENABLED")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(false),
            secret_manager_enabled,
            otp_provider: optional_env("NVBES_OTP_PROVIDER").unwrap_or_else(|| {
                if optional_env("NVBES_TWILIO_VERIFY_SERVICE_SID").is_some() {
                    "twilio_verify".to_string()
                } else {
                    "mock".to_string()
                }
            }),
            twilio_verify_service_sid: optional_env("NVBES_TWILIO_VERIFY_SERVICE_SID"),
            twilio_api_base_url: env_or_default(
                "NVBES_TWILIO_API_BASE_URL",
                std::env::var("NVBES_TWILIO_API_BASE_URL").ok(),
                "https://verify.twilio.com",
                false,
            )?,
            twilio_account_sid: optional_env("NVBES_TWILIO_ACCOUNT_SID"),
            twilio_auth_token: optional_env("NVBES_TWILIO_AUTH_TOKEN"),
            storage_enabled: std::env::var("STORAGE_ENABLED")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(false),
            storage_bucket: env_or_default(
                "STORAGE_BUCKET",
                std::env::var("STORAGE_BUCKET").ok(),
                "nvbes-drive",
                false,
            )?,
            storage_endpoint: optional_env("STORAGE_ENDPOINT"),
            storage_public_endpoint: optional_env("STORAGE_PUBLIC_ENDPOINT"),
            storage_region: optional_env("STORAGE_REGION").unwrap_or_else(|| "fr-par".to_string()),
            storage_access_key: optional_env("STORAGE_ACCESS_KEY")
                .or_else(|| optional_env("AWS_ACCESS_KEY_ID")),
            storage_secret_key: optional_env("STORAGE_SECRET_KEY")
                .or_else(|| optional_env("AWS_SECRET_ACCESS_KEY")),
            scan_enabled: std::env::var("SCAN_ENABLED")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(false),
            scan_engine: env_or_default(
                "SCAN_ENGINE",
                std::env::var("SCAN_ENGINE").ok(),
                if strict_mode { "clamav" } else { "mock" },
                false,
            )?,
            clamav_host: env_or_default(
                "CLAMAV_HOST",
                std::env::var("CLAMAV_HOST").ok(),
                "localhost",
                false,
            )?,
            clamav_port: std::env::var("CLAMAV_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(3310),
            scan_timeout_secs: std::env::var("SCAN_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(30),
            api_max_concurrent_requests: std::env::var("NVBES_API_MAX_CONCURRENT_REQUESTS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(50),
            api_max_connections_per_ip: std::env::var("NVBES_API_MAX_CONNECTIONS_PER_IP")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10),
            http_request_timeout_secs: std::env::var("NVBES_HTTP_REQUEST_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(30),
            quarantine_retention_days: std::env::var("QUARANTINE_RETENTION_DAYS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(30),
            scan_fail_open: std::env::var("SCAN_FAIL_OPEN")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(!strict_mode),
            trusted_proxy_cidrs: optional_env("NVBES_TRUSTED_PROXY_CIDRS")
                .map(|value| parse_csv(&value))
                .unwrap_or_default(),
            ip_intelligence_provider_specs: ip_intelligence_env.provider_specs,
            ip_intelligence_timeout_secs: ip_intelligence_env.timeout_secs,
            ip_intelligence_cache_ttl_hours: ip_intelligence_env.cache_ttl_hours,
            maxmind_geolite_database_enabled: env_bool(
                "NVBES_MAXMIND_GEOLITE_DATABASE_ENABLED",
                false,
            ),
            maxmind_geolite_web_enabled: env_bool("NVBES_MAXMIND_GEOLITE_WEB_ENABLED", false),
            maxmind_geolite_eula_accepted: env_bool("NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED", false),
            maxmind_account_id: optional_env("NVBES_MAXMIND_ACCOUNT_ID"),
            maxmind_license_key: optional_env("NVBES_MAXMIND_LICENSE_KEY"),
            maxmind_web_timeout_secs: std::env::var("NVBES_MAXMIND_WEB_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(2),
            maxmind_web_cache_ttl_hours: std::env::var("NVBES_MAXMIND_WEB_CACHE_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(24 * 7),
            loyalsoldier_geoip_enabled: env_bool("NVBES_LOYALSOLDIER_GEOIP_ENABLED", false),
            loyalsoldier_geoip_license_accepted: env_bool(
                "NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED",
                false,
            ),
            mtls_enabled: env_bool("NVBES_MTLS_ENABLED", false),
            mtls_port: std::env::var("NVBES_MTLS_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(api_port + 1),
            tls_cert_path: optional_env("NVBES_TLS_CERT_PATH"),
            tls_key_path: optional_env("NVBES_TLS_KEY_PATH"),
            tls_client_ca_path: optional_env("NVBES_TLS_CLIENT_CA_PATH"),
            mtls_client_cert_path: optional_env("NVBES_MTLS_CLIENT_CERT_PATH"),
            mtls_client_key_path: optional_env("NVBES_MTLS_CLIENT_KEY_PATH"),
            dpop_enabled: env_bool("NVBES_DPOP_ENABLED", false),
            fapi_high_assurance_enabled: env_bool("NVBES_FAPI_HIGH_ASSURANCE_ENABLED", false),
            fapi_conformance_evidence_sha256: optional_env(
                "NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256",
            ),
            request_e2ee_enabled: env_bool("NVBES_REQUEST_E2EE_ENABLED", false),
            request_e2ee_required: env_bool("NVBES_REQUEST_E2EE_REQUIRED", false),
            request_e2ee_key_id: optional_env("NVBES_REQUEST_E2EE_KEY_ID")
                .unwrap_or_else(|| "default".to_string()),
            request_e2ee_secret: optional_env("NVBES_REQUEST_E2EE_SECRET"),
            security_contact_email: optional_env("NVBES_SECURITY_CONTACT_EMAIL"),
        };

        if !secret_manager_enabled {
            validate_config_urls_and_secrets(&config)?;
        }
        Ok(config)
    }
}

#[cfg(test)]
#[path = "config.from_env.tests.rs"]
mod tests;
