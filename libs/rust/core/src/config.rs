#[path = "config.billing.rs"]
mod billing;
#[cfg(test)]
#[path = "config.tests.billing.rs"]
mod billing_tests;
#[path = "config.env.rs"]
mod env;
#[path = "config.from_env.rs"]
mod from_env;
#[path = "config.geo.rs"]
mod geo;
#[path = "config.secrets.rs"]
mod secrets;
#[cfg(test)]
#[path = "config.tests.rs"]
mod tests;
#[path = "config.validation.rs"]
mod validation;

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
    #[serde(skip_serializing)]
    pub billing_database_url: String,
    pub database_max_connections: u32,
    pub auth_session_ttl_hours: i64,
    pub auth_session_idle_ttl_minutes: i64,
    pub auth_refresh_token_ttl_hours: i64,
    pub auth_verification_ttl_hours: i64,
    pub auth_verification_resend_cooldown_seconds: i64,
    pub auth_unverified_account_ttl_days: i64,
    pub auth_password_reset_ttl_minutes: i64,
    pub auth_password_history_size: usize,
    #[serde(skip_serializing)]
    pub auth_password_pepper: Option<String>,
    #[serde(skip_serializing)]
    pub auth_factor_encryption_key: Option<String>,
    pub auth_factor_encryption_key_version: u32,
    pub auth_pow_enabled: bool,
    pub auth_pow_difficulty: u32,
    pub auth_pow_ttl_seconds: i64,
    pub auth_step_up_ttl_minutes: i64,
    pub auth_device_trust_ttl_days: i64,
    #[serde(skip_serializing)]
    pub stripe_secret_key: Option<String>,
    #[serde(skip_serializing)]
    pub stripe_webhook_secret: Option<String>,
    #[serde(skip_serializing)]
    pub jwt_secret: String,
    pub stripe_api_base_url: String,
    #[serde(skip_serializing)]
    pub mollie_api_key: Option<String>,
    pub mollie_api_base_url: String,
    pub billing_mollie_enabled: bool,
    pub billing_mollie_routing_status: String,
    pub billing_external_provider_fallback_enabled: bool,
    pub billing_external_provider_routing_status: String,
    pub billing_fraud_enforcement_enabled: bool,
    pub billing_fraud_step_up_threshold: u8,
    pub billing_fraud_manual_review_threshold: u8,
    pub billing_fraud_block_threshold: u8,
    pub billing_fraud_policy_overrides_json: Option<String>,
    pub billing_default_success_url: String,
    pub billing_default_cancel_url: String,
    pub billing_default_portal_return_url: String,
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,
    pub webauthn_related_origins: Vec<String>,
    #[serde(skip_serializing)]
    pub sentry_dsn: Option<String>,
    pub sentry_traces_sample_rate: f32,
    #[serde(skip_serializing)]
    pub otlp_endpoint: Option<String>,
    #[serde(skip_serializing)]
    pub otlp_authorization_header: Option<String>,
    pub product_analytics_enabled: bool,
    #[serde(skip_serializing)]
    pub product_analytics_token: Option<String>,
    pub posthog_host: String,
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
    pub secret_manager_enabled: bool,
    pub email_provider: String,
    pub email_from_email: Option<String>,
    pub email_from_name: Option<String>,
    pub email_reply_to: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    #[serde(skip_serializing)]
    pub smtp_password: Option<String>,
    pub smtp_starttls: bool,
    pub otp_provider: String,
    pub twilio_verify_service_sid: Option<String>,
    pub twilio_api_base_url: String,
    #[serde(skip_serializing)]
    pub twilio_account_sid: Option<String>,
    #[serde(skip_serializing)]
    pub twilio_auth_token: Option<String>,
    pub storage_enabled: bool,
    pub storage_bucket: String,
    pub storage_endpoint: Option<String>,
    pub storage_public_endpoint: Option<String>,
    pub storage_region: String,
    #[serde(skip_serializing)]
    pub storage_access_key: Option<String>,
    #[serde(skip_serializing)]
    pub storage_secret_key: Option<String>,
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
    pub ip_intelligence_provider_specs: Vec<String>,
    pub ip_intelligence_timeout_secs: u64,
    pub ip_intelligence_cache_ttl_hours: i64,
    pub maxmind_geolite_database_enabled: bool,
    pub maxmind_geolite_web_enabled: bool,
    pub maxmind_geolite_eula_accepted: bool,
    #[serde(skip_serializing)]
    pub maxmind_account_id: Option<String>,
    #[serde(skip_serializing)]
    pub maxmind_license_key: Option<String>,
    pub maxmind_web_timeout_secs: u64,
    pub maxmind_web_cache_ttl_hours: i64,
    pub loyalsoldier_geoip_enabled: bool,
    pub loyalsoldier_geoip_license_accepted: bool,
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
    pub fapi_high_assurance_enabled: bool,
    #[serde(skip_serializing)]
    pub fapi_conformance_evidence_sha256: Option<String>,
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
    pub fn billing_api_base_url(&self) -> String {
        std::env::var("NVBES_BILLING_SERVICE_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| self.api_base_url.clone())
    }

    pub fn billing_database_url(&self) -> &str {
        if self.billing_database_url.trim().is_empty() {
            &self.database_url
        } else {
            &self.billing_database_url
        }
    }
}
