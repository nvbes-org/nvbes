#[path = "config.billing.rs"]
mod billing;
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
    #[serde(skip_serializing)]
    pub mollie_api_key: Option<String>,
    pub mollie_api_base_url: String,
    pub billing_mollie_enabled: bool,
    pub billing_mollie_routing_status: String,
    pub billing_external_provider_fallback_enabled: bool,
    pub billing_external_provider_routing_status: String,
    pub billing_default_success_url: String,
    pub billing_default_cancel_url: String,
    pub billing_default_portal_return_url: String,
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,
    pub webauthn_related_origins: Vec<String>,
    #[serde(skip_serializing)]
    pub otlp_endpoint: Option<String>,
    #[serde(skip_serializing)]
    pub otlp_authorization_header: Option<String>,
    pub product_analytics_enabled: bool,
    #[serde(skip_serializing)]
    pub product_analytics_token: Option<String>,
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
