use super::AppConfig;
use super::billing::billing_provider_env;
use super::env::{env_bool, env_or_default, optional_env, parse_csv};
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

        let api_port = std::env::var("NVBES_API_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(4000);

        let billing_provider_env = billing_provider_env()?;

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
                strict_mode,
            )?,
            database_max_connections: std::env::var("NVBES_DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10),
            auth_session_ttl_hours: std::env::var("NVBES_AUTH_SESSION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(24 * 30),
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
            auth_password_max_age_days: std::env::var("NVBES_AUTH_PASSWORD_MAX_AGE_DAYS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok()),
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
            stripe_secret_key: billing_provider_env.stripe_secret_key,
            stripe_webhook_secret: billing_provider_env.stripe_webhook_secret,
            stripe_api_base_url: billing_provider_env.stripe_api_base_url,
            mollie_api_key: billing_provider_env.mollie_api_key,
            mollie_api_base_url: billing_provider_env.mollie_api_base_url,
            billing_mollie_enabled: billing_provider_env.billing_mollie_enabled,
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
                strict_mode,
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
            otlp_endpoint: optional_env("NVBES_OTLP_ENDPOINT"),
            otlp_authorization_header: optional_env("NVBES_OTLP_AUTHORIZATION_HEADER"),
            product_analytics_enabled: env_bool("NVBES_PRODUCT_ANALYTICS_ENABLED", false),
            product_analytics_token: optional_env("NVBES_PRODUCT_ANALYTICS_TOKEN"),
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
            secret_manager_enabled: std::env::var("NVBES_SECRET_MANAGER_ENABLED")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(false),
            email_provider: optional_env("NVBES_EMAIL_PROVIDER").unwrap_or_else(|| {
                if optional_env("NVBES_SMTP_HOST").is_some() {
                    "smtp".to_string()
                } else {
                    "mock".to_string()
                }
            }),
            email_from_email: optional_env("NVBES_EMAIL_FROM_EMAIL"),
            email_from_name: optional_env("NVBES_EMAIL_FROM_NAME"),
            email_reply_to: optional_env("NVBES_EMAIL_REPLY_TO"),
            smtp_host: optional_env("NVBES_SMTP_HOST"),
            smtp_port: std::env::var("NVBES_SMTP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(587),
            smtp_username: optional_env("NVBES_SMTP_USERNAME"),
            smtp_password: optional_env("NVBES_SMTP_PASSWORD"),
            smtp_starttls: std::env::var("NVBES_SMTP_STARTTLS")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(true),
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
            request_e2ee_enabled: env_bool("NVBES_REQUEST_E2EE_ENABLED", false),
            request_e2ee_required: env_bool("NVBES_REQUEST_E2EE_REQUIRED", false),
            request_e2ee_key_id: optional_env("NVBES_REQUEST_E2EE_KEY_ID")
                .unwrap_or_else(|| "default".to_string()),
            request_e2ee_secret: optional_env("NVBES_REQUEST_E2EE_SECRET"),
            redis_url: env_or_default(
                "NVBES_REDIS_URL",
                std::env::var("NVBES_REDIS_URL").ok(),
                "redis://localhost:6379",
                strict_mode,
            )?,
            redis_password: optional_env("NVBES_REDIS_PASSWORD"),
            redis_max_connections: std::env::var("NVBES_REDIS_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10),
            security_contact_email: optional_env("NVBES_SECURITY_CONTACT_EMAIL"),
        };

        validate_config_urls_and_secrets(&config)?;
        Ok(config)
    }
}
