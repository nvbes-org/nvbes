use reqwest::Url;
use std::net::IpAddr;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AppConfig {
    pub app_name: String,
    pub environment: String,
    pub api_port: u16,
    pub web_base_url: String,
    pub api_base_url: String,
    pub staging_web_base_url: Option<String>,
    pub staging_api_base_url: Option<String>,
    #[serde(skip_serializing)]
    pub additional_cors_origins: Vec<String>,
    #[serde(skip_serializing)]
    pub database_url: String,
    pub database_max_connections: u32,
    pub auth_session_ttl_hours: i64,
    pub auth_refresh_token_ttl_hours: i64,
    pub auth_verification_ttl_hours: i64,
    pub auth_verification_resend_cooldown_seconds: i64,
    pub auth_unverified_account_ttl_days: i64,
    pub auth_password_reset_ttl_minutes: i64,
    pub auth_password_history_size: usize,
    pub auth_password_max_age_days: Option<i64>,
    pub auth_pow_enabled: bool,
    pub auth_pow_difficulty: u32,
    pub auth_pow_ttl_seconds: i64,
    pub auth_step_up_ttl_minutes: i64,
    #[serde(skip_serializing)]
    pub stripe_secret_key: Option<String>,
    #[serde(skip_serializing)]
    pub stripe_webhook_secret: Option<String>,
    #[serde(skip_serializing)]
    pub jwt_secret: String,
    pub stripe_api_base_url: String,
    pub billing_default_success_url: String,
    pub billing_default_cancel_url: String,
    pub billing_default_portal_return_url: String,
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,
    pub webauthn_related_origins: Vec<String>,
    #[serde(skip_serializing)]
    pub sentry_dsn: Option<String>,
    pub sentry_logs_enabled: bool,
    #[serde(skip_serializing)]
    pub otlp_endpoint: Option<String>,
    #[serde(skip_serializing)]
    pub otlp_authorization_header: Option<String>,
    pub posthog_enabled: bool,
    pub posthog_host: String,
    #[serde(skip_serializing)]
    pub posthog_project_token: Option<String>,
    #[serde(skip_serializing)]
    pub analytics_id_salt: Option<String>,
    pub profiling_enabled: bool,
    #[serde(skip_serializing)]
    pub profiling_endpoint: Option<String>,
    pub profiling_sample_rate_hz: u32,
    #[serde(skip_serializing)]
    pub profiling_basic_auth_user: Option<String>,
    #[serde(skip_serializing)]
    pub profiling_basic_auth_password: Option<String>,
    #[serde(skip_serializing)]
    pub observability_internal_token: Option<String>,
    pub kms_enabled: bool,
    #[serde(skip_serializing)]
    pub scw_access_key: Option<String>,
    #[serde(skip_serializing)]
    pub scw_secret_key: Option<String>,
    #[serde(skip_serializing)]
    pub scw_project_id: Option<String>,
    pub scw_region: String,
    #[serde(skip_serializing)]
    pub scw_kms_key_id: Option<String>,
    pub secret_manager_enabled: bool,
    pub scw_tem_enabled: bool,
    #[serde(skip_serializing)]
    pub scw_tem_from_email: Option<String>,
    pub scw_tem_from_name: Option<String>,
    #[serde(skip_serializing)]
    pub scw_tem_reply_to: Option<String>,
    #[serde(skip_serializing)]
    pub scw_tem_webhook_secret: Option<String>,
    pub storage_enabled: bool,
    pub storage_bucket: String,
    pub storage_endpoint: Option<String>,
    pub storage_region: String,
    pub scan_enabled: bool,
    pub scan_engine: String,
    pub clamav_host: String,
    pub clamav_port: u16,
    pub scan_timeout_secs: u64,
    pub api_max_concurrent_requests: u32,
    pub api_max_connections_per_ip: u32,
    pub http_request_timeout_secs: u64,
    pub quarantine_retention_days: u32,
    pub scan_fail_open: bool,
    pub trusted_proxy_cidrs: Vec<String>,
    #[serde(skip_serializing)]
    pub turnstile_secret_key: Option<String>,
    pub mtls_enabled: bool,
    pub mtls_port: u16,
    #[serde(skip_serializing)]
    pub tls_cert_path: Option<String>,
    #[serde(skip_serializing)]
    pub tls_key_path: Option<String>,
    #[serde(skip_serializing)]
    pub tls_client_ca_path: Option<String>,
    #[serde(skip_serializing)]
    pub mtls_client_cert_path: Option<String>,
    #[serde(skip_serializing)]
    pub mtls_client_key_path: Option<String>,
    pub dpop_enabled: bool,
    pub request_e2ee_enabled: bool,
    pub request_e2ee_required: bool,
    pub request_e2ee_key_id: String,
    #[serde(skip_serializing)]
    pub request_e2ee_secret: Option<String>,
    #[serde(skip_serializing)]
    pub redis_url: String,
    #[serde(skip_serializing)]
    pub redis_password: Option<String>,
    pub redis_max_connections: u32,
    pub security_contact_email: Option<String>,
}

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
            stripe_secret_key: optional_env("NVBES_STRIPE_SECRET_KEY"),
            stripe_webhook_secret: optional_env("NVBES_STRIPE_WEBHOOK_SECRET"),
            stripe_api_base_url: env_or_default(
                "NVBES_STRIPE_API_BASE_URL",
                std::env::var("NVBES_STRIPE_API_BASE_URL").ok(),
                "https://api.stripe.com",
                false,
            )?,
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
            sentry_dsn: optional_env("NVBES_SENTRY_DSN"),
            sentry_logs_enabled: env_bool("NVBES_SENTRY_LOGS_ENABLED", false),
            otlp_endpoint: optional_env("NVBES_OTLP_ENDPOINT"),
            otlp_authorization_header: optional_env("NVBES_OTLP_AUTHORIZATION_HEADER"),
            posthog_enabled: env_bool("NVBES_POSTHOG_ENABLED", false),
            posthog_host: optional_env("NVBES_POSTHOG_HOST")
                .unwrap_or_else(|| "https://eu.i.posthog.com".to_string()),
            posthog_project_token: optional_env("NVBES_POSTHOG_PROJECT_TOKEN"),
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
            scw_access_key: optional_env("SCW_ACCESS_KEY"),
            scw_secret_key: optional_env("SCW_SECRET_KEY"),
            scw_project_id: optional_env("SCW_DEFAULT_PROJECT_ID")
                .or_else(|| optional_env("SCW_PROJECT_ID")),
            scw_region: optional_env("SCW_DEFAULT_REGION")
                .or_else(|| optional_env("SCW_REGION"))
                .unwrap_or_else(|| "fr-par".to_string()),
            scw_kms_key_id: optional_env("SCW_KMS_KEY_ID"),
            secret_manager_enabled: std::env::var("NVBES_SECRET_MANAGER_ENABLED")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(false),
            scw_tem_enabled: std::env::var("SCW_TEM_ENABLED")
                .ok()
                .map(|v| v == "true")
                .unwrap_or(false),
            scw_tem_from_email: optional_env("SCW_TEM_FROM_EMAIL"),
            scw_tem_from_name: optional_env("SCW_TEM_FROM_NAME"),
            scw_tem_reply_to: optional_env("SCW_TEM_REPLY_TO"),
            scw_tem_webhook_secret: optional_env("SCW_TEM_WEBHOOK_SECRET"),
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
            turnstile_secret_key: optional_env("TURNSTILE_SECRET_KEY"),
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

    pub async fn resolve_from_secret_manager(&mut self) -> Result<(), String> {
        if !self.secret_manager_enabled {
            return Ok(());
        }

        let scw_config = crate::scw_kms::KmsConfig {
            access_key: self.scw_access_key.clone().ok_or_else(|| {
                "SCW_ACCESS_KEY required when secret_manager_enabled is true".to_string()
            })?,
            secret_key: self.scw_secret_key.clone().ok_or_else(|| {
                "SCW_SECRET_KEY required when secret_manager_enabled is true".to_string()
            })?,
            project_id: self.scw_project_id.clone().ok_or_else(|| {
                "SCW_DEFAULT_PROJECT_ID required when secret_manager_enabled is true".to_string()
            })?,
            region: self.scw_region.clone(),
            key_id: self.scw_kms_key_id.clone(),
            enabled: true,
        };

        let resolver = crate::scw_secrets::SecretResolver::new(&scw_config, &self.environment);
        let secrets = resolver.resolve_secrets().await?;

        if let Some(v) = secrets.get("NVBES_DATABASE_URL") {
            self.database_url = v.clone();
        }
        if let Some(v) = secrets.get("NVBES_JWT_SECRET") {
            self.jwt_secret = v.clone();
        }
        if let Some(v) = secrets.get("NVBES_STRIPE_SECRET_KEY") {
            self.stripe_secret_key = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_STRIPE_WEBHOOK_SECRET") {
            self.stripe_webhook_secret = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_SENTRY_DSN") {
            self.sentry_dsn = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_OTLP_ENDPOINT") {
            self.otlp_endpoint = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_OTLP_AUTHORIZATION_HEADER") {
            self.otlp_authorization_header = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_POSTHOG_PROJECT_TOKEN") {
            self.posthog_project_token = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_ANALYTICS_ID_SALT") {
            self.analytics_id_salt = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_PROFILING_ENDPOINT") {
            self.profiling_endpoint = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_PROFILING_BASIC_AUTH_USER") {
            self.profiling_basic_auth_user = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_PROFILING_BASIC_AUTH_PASSWORD") {
            self.profiling_basic_auth_password = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_OBSERVABILITY_INTERNAL_TOKEN") {
            self.observability_internal_token = Some(v.clone());
        }
        if let Some(v) = secrets.get("SCW_TEM_FROM_EMAIL") {
            self.scw_tem_from_email = Some(v.clone());
        }
        if let Some(v) = secrets.get("SCW_TEM_WEBHOOK_SECRET") {
            self.scw_tem_webhook_secret = Some(v.clone());
        }
        if let Some(v) = secrets.get("TURNSTILE_SECRET_KEY") {
            self.turnstile_secret_key = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_REQUEST_E2EE_SECRET") {
            self.request_e2ee_secret = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_REDIS_URL") {
            self.redis_url = v.clone();
        }
        if let Some(v) = secrets.get("NVBES_REDIS_PASSWORD") {
            self.redis_password = Some(v.clone());
        }

        validate_config_urls_and_secrets(self)?;
        tracing::info!("Config resolved from Scaleway Secret Manager");
        Ok(())
    }
}

fn env_or_default(
    name: &str,
    value: Option<String>,
    default: &str,
    strict_mode: bool,
) -> Result<String, String> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        _ if strict_mode => Err(format!(
            "{name} must be set when NVBES_ENV is not development"
        )),
        _ => Ok(default.to_string()),
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}

fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn validate_config_urls_and_secrets(config: &AppConfig) -> Result<(), String> {
    let strict_mode = config.environment != "development";

    if config.mtls_enabled {
        if config.tls_cert_path.as_deref().unwrap_or("").is_empty() {
            return Err(
                "NVBES_TLS_CERT_PATH is required when NVBES_MTLS_ENABLED is true".to_string(),
            );
        }
        if config.tls_key_path.as_deref().unwrap_or("").is_empty() {
            return Err(
                "NVBES_TLS_KEY_PATH is required when NVBES_MTLS_ENABLED is true".to_string(),
            );
        }
        if config
            .tls_client_ca_path
            .as_deref()
            .unwrap_or("")
            .is_empty()
        {
            return Err(
                "NVBES_TLS_CLIENT_CA_PATH is required when NVBES_MTLS_ENABLED is true".to_string(),
            );
        }
    }

    validate_public_url(
        "NVBES_WEB_BASE_URL",
        &config.web_base_url,
        strict_mode,
        true,
    )?;
    validate_public_url(
        "NVBES_API_BASE_URL",
        &config.api_base_url,
        strict_mode,
        true,
    )?;
    validate_public_url(
        "NVBES_BILLING_SUCCESS_URL",
        &config.billing_default_success_url,
        strict_mode,
        true,
    )?;
    validate_public_url(
        "NVBES_BILLING_CANCEL_URL",
        &config.billing_default_cancel_url,
        strict_mode,
        true,
    )?;
    validate_public_url(
        "NVBES_BILLING_PORTAL_RETURN_URL",
        &config.billing_default_portal_return_url,
        strict_mode,
        true,
    )?;
    validate_public_url(
        "NVBES_WEBAUTHN_RP_ORIGIN",
        &config.webauthn_rp_origin,
        strict_mode,
        true,
    )?;
    validate_webauthn_rp_id(&config.webauthn_rp_id, strict_mode)?;
    validate_public_url(
        "NVBES_STRIPE_API_BASE_URL",
        &config.stripe_api_base_url,
        strict_mode,
        true,
    )?;
    validate_database_url(&config.database_url, strict_mode)?;
    validate_jwt_secret(&config.jwt_secret, strict_mode)?;
    validate_grafana_export_path(config, strict_mode)?;
    validate_posthog_analytics(config, strict_mode)?;
    validate_profiling(config)?;
    validate_observability_internal_token(config, strict_mode)?;
    validate_positive_integer(
        "NVBES_AUTH_VERIFICATION_RESEND_COOLDOWN_SECONDS",
        config.auth_verification_resend_cooldown_seconds,
    )?;
    validate_positive_integer(
        "NVBES_AUTH_UNVERIFIED_ACCOUNT_TTL_DAYS",
        config.auth_unverified_account_ttl_days,
    )?;
    validate_request_e2ee(config, strict_mode)?;
    Ok(())
}

fn validate_database_url(value: &str, strict_mode: bool) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| "DATABASE_URL must be a valid URL".to_string())?;

    if !strict_mode {
        return Ok(());
    }

    let sslmode = url.query_pairs().find_map(|(key, value)| {
        if key == "sslmode" {
            Some(value.into_owned())
        } else {
            None
        }
    });

    match sslmode.as_deref() {
        Some("require") | Some("verify-full") => Ok(()),
        Some(other) => Err(format!(
            "DATABASE_URL sslmode must be require or verify-full outside development, got '{other}'"
        )),
        None => Err(
            "DATABASE_URL must specify sslmode=require or sslmode=verify-full outside development"
                .to_string(),
        ),
    }
}

fn validate_public_url(
    name: &str,
    value: &str,
    strict_mode: bool,
    require_https: bool,
) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| format!("{name} must be a valid URL"))?;

    if !strict_mode {
        return Ok(());
    }

    if require_https && url.scheme() != "https" {
        return Err(format!("{name} must use HTTPS outside development"));
    }

    let host = url
        .host_str()
        .ok_or_else(|| format!("{name} must include a host"))?;

    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err(format!(
            "{name} cannot point to localhost outside development"
        ));
    }

    if let Ok(ip_addr) = host.parse::<IpAddr>() {
        let is_loopback = match ip_addr {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        };

        if is_loopback {
            return Err(format!(
                "{name} cannot point to loopback addresses outside development"
            ));
        }
    }

    Ok(())
}

fn validate_jwt_secret(value: &str, strict_mode: bool) -> Result<(), String> {
    if !strict_mode {
        return Ok(());
    }

    if value == "default-secret-change-me" {
        return Err(
            "NVBES_JWT_SECRET cannot use the placeholder secret outside development".to_string(),
        );
    }

    if value.len() < 32 {
        return Err(
            "NVBES_JWT_SECRET must be at least 32 characters long outside development".to_string(),
        );
    }

    Ok(())
}

fn validate_observability_internal_token(
    config: &AppConfig,
    strict_mode: bool,
) -> Result<(), String> {
    let Some(token) = config.observability_internal_token.as_deref() else {
        return if strict_mode {
            Err("NVBES_OBSERVABILITY_INTERNAL_TOKEN is required outside development".to_string())
        } else {
            Ok(())
        };
    };

    if token.len() < 32 {
        return Err(
            "NVBES_OBSERVABILITY_INTERNAL_TOKEN must be at least 32 characters long".to_string(),
        );
    }

    let normalized = token.to_ascii_lowercase();
    if normalized == "default-secret-change-me" || normalized.contains("change-me") {
        return Err(
            "NVBES_OBSERVABILITY_INTERNAL_TOKEN cannot use a placeholder value".to_string(),
        );
    }

    Ok(())
}

fn validate_profiling(config: &AppConfig) -> Result<(), String> {
    if config.profiling_sample_rate_hz == 0 {
        return Err("NVBES_PROFILING_SAMPLE_RATE_HZ must be greater than zero".to_string());
    }

    if !config.profiling_enabled {
        return Ok(());
    }

    let endpoint = config.profiling_endpoint.as_deref().ok_or_else(|| {
        "NVBES_PROFILING_ENDPOINT is required when profiling is enabled".to_string()
    })?;

    validate_profiling_endpoint(endpoint)?;

    match (
        &config.profiling_basic_auth_user,
        &config.profiling_basic_auth_password,
    ) {
        (Some(_), Some(_)) | (None, None) => Ok(()),
        _ => Err(
            "NVBES_PROFILING_BASIC_AUTH_USER and NVBES_PROFILING_BASIC_AUTH_PASSWORD must be set together"
                .to_string(),
        ),
    }
}

fn validate_grafana_export_path(config: &AppConfig, strict_mode: bool) -> Result<(), String> {
    if !strict_mode {
        return Ok(());
    }

    if config.otlp_authorization_header.is_some() {
        return Err(
            "NVBES_OTLP_AUTHORIZATION_HEADER must stay empty outside development; export OTLP through local Grafana Alloy"
                .to_string(),
        );
    }

    if config.profiling_basic_auth_user.is_some() || config.profiling_basic_auth_password.is_some()
    {
        return Err(
            "NVBES_PROFILING_BASIC_AUTH_* must stay empty outside development; export profiles through local Grafana Alloy"
                .to_string(),
        );
    }

    Ok(())
}

fn validate_posthog_analytics(config: &AppConfig, strict_mode: bool) -> Result<(), String> {
    if !config.posthog_enabled && config.posthog_host.trim().is_empty() {
        return Ok(());
    }

    let url = Url::parse(&config.posthog_host)
        .map_err(|_| "NVBES_POSTHOG_HOST must be a valid URL".to_string())?;
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("NVBES_POSTHOG_HOST must use HTTP or HTTPS".to_string()),
    }

    if !config.posthog_enabled {
        return Ok(());
    }

    if config
        .posthog_project_token
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err(
            "NVBES_POSTHOG_PROJECT_TOKEN is required when NVBES_POSTHOG_ENABLED is true"
                .to_string(),
        );
    }

    let salt = config.analytics_id_salt.as_deref().unwrap_or("").trim();
    if salt.is_empty() {
        return Err(
            "NVBES_ANALYTICS_ID_SALT is required when NVBES_POSTHOG_ENABLED is true".to_string(),
        );
    }
    if strict_mode && salt.len() < 32 {
        return Err(
            "NVBES_ANALYTICS_ID_SALT must be at least 32 characters outside development"
                .to_string(),
        );
    }

    if strict_mode {
        let host = url.host_str().unwrap_or("").to_ascii_lowercase();
        if host == "app.posthog.com" || host == "us.i.posthog.com" {
            return Err(
                "NVBES_POSTHOG_HOST must use PostHog EU Cloud or a first-party proxy outside development"
                    .to_string(),
            );
        }
    }

    Ok(())
}

fn validate_profiling_endpoint(value: &str) -> Result<(), String> {
    let url = Url::parse(value)
        .map_err(|_| "NVBES_PROFILING_ENDPOINT must be a valid URL".to_string())?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("NVBES_PROFILING_ENDPOINT must use HTTP or HTTPS".to_string()),
    }

    url.host_str()
        .ok_or_else(|| "NVBES_PROFILING_ENDPOINT must include a host".to_string())?;

    Ok(())
}

fn validate_webauthn_rp_id(value: &str, strict_mode: bool) -> Result<(), String> {
    let value = value.trim();

    if value.is_empty() {
        return Err("NVBES_WEBAUTHN_RP_ID must not be empty".to_string());
    }

    if !strict_mode {
        return Ok(());
    }

    if value.eq_ignore_ascii_case("localhost") {
        return Err("NVBES_WEBAUTHN_RP_ID cannot be localhost outside development".to_string());
    }

    if let Ok(ip_addr) = value.parse::<IpAddr>() {
        let is_loopback = match ip_addr {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        };

        if is_loopback {
            return Err(
                "NVBES_WEBAUTHN_RP_ID cannot point to loopback addresses outside development"
                    .to_string(),
            );
        }
    }

    Ok(())
}

fn validate_positive_integer(name: &str, value: i64) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("{name} must be greater than zero"));
    }

    Ok(())
}

fn validate_request_e2ee(config: &AppConfig, strict_mode: bool) -> Result<(), String> {
    if config.request_e2ee_required && !config.request_e2ee_enabled {
        return Err(
            "NVBES_REQUEST_E2EE_ENABLED must be true when NVBES_REQUEST_E2EE_REQUIRED is true"
                .to_string(),
        );
    }

    if !config.request_e2ee_enabled {
        return Ok(());
    }

    let secret = config.request_e2ee_secret.as_deref().ok_or_else(|| {
        "NVBES_REQUEST_E2EE_SECRET is required when request E2EE is enabled".to_string()
    })?;

    if secret.len() < 32 {
        return Err("NVBES_REQUEST_E2EE_SECRET must be at least 32 characters long".to_string());
    }

    if config.request_e2ee_key_id.trim().is_empty() {
        return Err("NVBES_REQUEST_E2EE_KEY_ID must not be empty".to_string());
    }

    if strict_mode && config.request_e2ee_key_id == "default" {
        return Err(
            "NVBES_REQUEST_E2EE_KEY_ID cannot be 'default' outside development".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, env_or_default, validate_grafana_export_path, validate_jwt_secret,
        validate_observability_internal_token, validate_positive_integer,
        validate_posthog_analytics, validate_profiling, validate_profiling_endpoint,
        validate_public_url, validate_request_e2ee, validate_webauthn_rp_id,
    };

    #[test]
    fn env_or_default_uses_dev_fallbacks() {
        let value = env_or_default("NVBES_JWT_SECRET", None, "fallback", false)
            .expect("expected development fallback");
        assert_eq!(value, "fallback");
    }

    #[test]
    fn env_or_default_rejects_missing_value_in_strict_mode() {
        let error = env_or_default("NVBES_JWT_SECRET", None, "fallback", true)
            .expect_err("expected strict mode to reject missing env");
        assert!(error.contains("NVBES_JWT_SECRET"));
    }

    #[test]
    fn validate_public_url_rejects_localhost_in_strict_mode() {
        let error = validate_public_url("NVBES_WEB_BASE_URL", "https://localhost:5173", true, true)
            .expect_err("expected localhost URL to be rejected");
        assert!(error.contains("localhost"));
    }

    #[test]
    fn validate_public_url_allows_localhost_in_development() {
        validate_public_url("NVBES_WEB_BASE_URL", "http://localhost:5173", false, true)
            .expect("expected development localhost URL to be accepted");
    }

    #[test]
    fn validate_jwt_secret_rejects_placeholder_and_short_values() {
        let placeholder = validate_jwt_secret("default-secret-change-me", true)
            .expect_err("expected placeholder secret to be rejected");
        assert!(placeholder.contains("placeholder"));

        let short = validate_jwt_secret("too-short-secret", true)
            .expect_err("expected short secret to be rejected");
        assert!(short.contains("32"));
    }

    #[test]
    fn validate_observability_internal_token_is_required_in_strict_mode() {
        let config = AppConfig::default();

        let error = validate_observability_internal_token(&config, true)
            .expect_err("strict mode must require an internal observability token");

        assert!(error.contains("NVBES_OBSERVABILITY_INTERNAL_TOKEN"));
    }

    #[test]
    fn validate_observability_internal_token_rejects_weak_values() {
        let short = AppConfig {
            observability_internal_token: Some("short".to_string()),
            ..AppConfig::default()
        };
        let short_error = validate_observability_internal_token(&short, false)
            .expect_err("short observability token must be rejected");
        assert!(short_error.contains("32"));

        let placeholder = AppConfig {
            observability_internal_token: Some("change-me-change-me-change-me-1234".to_string()),
            ..AppConfig::default()
        };
        let placeholder_error = validate_observability_internal_token(&placeholder, true)
            .expect_err("placeholder observability token must be rejected");
        assert!(placeholder_error.contains("placeholder"));
    }

    #[test]
    fn validate_profiling_requires_endpoint_when_enabled() {
        let config = AppConfig {
            profiling_enabled: true,
            profiling_sample_rate_hz: 100,
            ..AppConfig::default()
        };

        let error =
            validate_profiling(&config).expect_err("enabled profiling must require an endpoint");

        assert!(error.contains("NVBES_PROFILING_ENDPOINT"));
    }

    #[test]
    fn validate_profiling_rejects_partial_basic_auth() {
        let config = AppConfig {
            profiling_enabled: true,
            profiling_endpoint: Some("http://127.0.0.1:4040".to_string()),
            profiling_sample_rate_hz: 100,
            profiling_basic_auth_user: Some("tenant".to_string()),
            profiling_basic_auth_password: None,
            ..AppConfig::default()
        };

        let error =
            validate_profiling(&config).expect_err("partial profiling basic auth must be rejected");

        assert!(error.contains("must be set together"));
    }

    #[test]
    fn validate_profiling_endpoint_allows_local_alloy() {
        validate_profiling_endpoint("http://127.0.0.1:4040")
            .expect("local Alloy Pyroscope endpoint must be accepted");
    }

    #[test]
    fn validate_grafana_export_path_rejects_direct_otlp_auth_outside_development() {
        let config = AppConfig {
            otlp_authorization_header: Some("Basic direct-grafana-cloud-token".to_string()),
            ..AppConfig::default()
        };

        let error = validate_grafana_export_path(&config, true)
            .expect_err("strict mode must reject direct OTLP auth");

        assert!(error.contains("Grafana Alloy"));
    }

    #[test]
    fn validate_grafana_export_path_rejects_direct_profile_auth_outside_development() {
        let config = AppConfig {
            profiling_basic_auth_user: Some("stack".to_string()),
            profiling_basic_auth_password: Some("token".to_string()),
            ..AppConfig::default()
        };

        let error = validate_grafana_export_path(&config, true)
            .expect_err("strict mode must reject direct Pyroscope auth");

        assert!(error.contains("Grafana Alloy"));
    }

    #[test]
    fn validate_posthog_requires_token_and_salt_when_enabled() {
        let config = AppConfig {
            posthog_enabled: true,
            posthog_host: "https://eu.i.posthog.com".to_string(),
            ..AppConfig::default()
        };

        let error = validate_posthog_analytics(&config, false)
            .expect_err("enabled PostHog must require a project token");

        assert!(error.contains("NVBES_POSTHOG_PROJECT_TOKEN"));
    }

    #[test]
    fn validate_posthog_rejects_us_direct_host_in_strict_mode() {
        let config = AppConfig {
            posthog_enabled: true,
            posthog_host: "https://us.i.posthog.com".to_string(),
            posthog_project_token: Some("phc_test".to_string()),
            analytics_id_salt: Some("01234567890123456789012345678901".to_string()),
            ..AppConfig::default()
        };

        let error = validate_posthog_analytics(&config, true)
            .expect_err("strict mode must reject PostHog US direct host");

        assert!(error.contains("EU Cloud"));
    }

    #[test]
    fn validate_webauthn_rp_id_rejects_localhost_in_strict_mode() {
        let error = validate_webauthn_rp_id("localhost", true)
            .expect_err("expected localhost rp id to be rejected");
        assert!(error.contains("localhost"));
    }

    #[test]
    fn validate_positive_integer_rejects_zero_and_negative_values() {
        let zero = validate_positive_integer("NVBES_AUTH_VERIFICATION_RESEND_COOLDOWN_SECONDS", 0)
            .expect_err("expected zero to be rejected");
        assert!(zero.contains("greater than zero"));

        let negative = validate_positive_integer("NVBES_AUTH_UNVERIFIED_ACCOUNT_TTL_DAYS", -1)
            .expect_err("expected negative to be rejected");
        assert!(negative.contains("greater than zero"));
    }

    #[test]
    fn validate_request_e2ee_rejects_required_without_enabled() {
        let config = AppConfig {
            request_e2ee_required: true,
            ..AppConfig::default()
        };

        let error =
            validate_request_e2ee(&config, false).expect_err("required mode must also enable E2EE");

        assert!(error.contains("NVBES_REQUEST_E2EE_ENABLED"));
    }

    #[test]
    fn validate_request_e2ee_requires_strong_secret() {
        let config = AppConfig {
            request_e2ee_enabled: true,
            request_e2ee_key_id: "kid_01".to_string(),
            request_e2ee_secret: Some("short".to_string()),
            ..AppConfig::default()
        };

        let error =
            validate_request_e2ee(&config, false).expect_err("short E2EE secret must be rejected");

        assert!(error.contains("32"));
    }
}
