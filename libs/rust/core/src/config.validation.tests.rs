use super::validate_config_urls_and_secrets;
use crate::config::AppConfig;

fn strict_production_config() -> AppConfig {
    AppConfig {
        environment: "production".to_string(),
        web_base_url: "https://app.example.com".to_string(),
        api_base_url: "https://api.example.com".to_string(),
        billing_default_success_url: "https://app.example.com/billing/success".to_string(),
        billing_default_cancel_url: "https://app.example.com/billing/cancel".to_string(),
        billing_default_portal_return_url: "https://app.example.com/billing".to_string(),
        webauthn_rp_origin: "https://app.example.com".to_string(),
        webauthn_rp_id: "app.example.com".to_string(),
        stripe_api_base_url: "https://api.stripe.com".to_string(),
        mollie_api_base_url: "https://api.mollie.com".to_string(),
        twilio_api_base_url: "https://verify.twilio.com".to_string(),
        database_url: "postgres://postgres:postgres@db.example.com:5432/nvbes?sslmode=require"
            .to_string(),
        billing_database_url:
            "postgres://postgres:postgres@db.example.com:5432/billing?sslmode=require".to_string(),
        jwt_secret: "a-very-long-production-jwt-secret-value".to_string(),
        observability_internal_token: Some("a-very-long-observability-token-value".to_string()),
        otp_provider: "mock".to_string(),
        ..AppConfig::default()
    }
}

fn mtls_config_with_paths() -> AppConfig {
    AppConfig {
        mtls_enabled: true,
        tls_cert_path: Some("/tmp/cert.pem".into()),
        tls_key_path: Some("/tmp/key.pem".into()),
        tls_client_ca_path: Some("/tmp/ca.pem".into()),
        ..AppConfig::default()
    }
}

#[test]
fn validate_config_mtls_requires_cert_path() {
    let mut config = mtls_config_with_paths();
    config.tls_cert_path = None;
    let err = validate_config_urls_and_secrets(&config).expect_err("mtls validation");
    assert!(err.contains("NVBES_TLS_CERT_PATH"));
}

#[test]
fn validate_config_mtls_requires_key_path() {
    let mut config = mtls_config_with_paths();
    config.tls_key_path = None;
    let err = validate_config_urls_and_secrets(&config).expect_err("mtls validation");
    assert!(err.contains("NVBES_TLS_KEY_PATH"));
}

#[test]
fn validate_config_mtls_requires_client_ca_path() {
    let mut config = mtls_config_with_paths();
    config.tls_client_ca_path = None;
    let err = validate_config_urls_and_secrets(&config).expect_err("mtls validation");
    assert!(err.contains("NVBES_TLS_CLIENT_CA_PATH"));
}

#[test]
fn validate_config_rejects_unsupported_otp_provider() {
    let mut config = strict_production_config();
    config.environment = "development".to_string();
    config.otp_provider = "sms_gateway".to_string();
    let err = validate_config_urls_and_secrets(&config).expect_err("unsupported otp");
    assert!(err.contains("Unsupported NVBES_OTP_PROVIDER"));
}

#[test]
fn validate_config_requires_mollie_api_key_when_enabled() {
    let mut config = strict_production_config();
    config.environment = "development".to_string();
    config.otp_provider = "mock".to_string();
    config.billing_mollie_enabled = true;
    let err = validate_config_urls_and_secrets(&config).expect_err("mollie key");
    assert!(err.contains("NVBES_MOLLIE_API_KEY"));
}

#[test]
fn validate_config_requires_stripe_key_for_external_fallback() {
    let mut config = strict_production_config();
    config.environment = "development".to_string();
    config.otp_provider = "mock".to_string();
    config.billing_external_provider_fallback_enabled = true;
    let err = validate_config_urls_and_secrets(&config).expect_err("stripe key");
    assert!(err.contains("NVBES_STRIPE_SECRET_KEY"));
}

#[test]
fn validate_config_rejects_invalid_fraud_threshold_ordering() {
    let mut config = strict_production_config();
    config.environment = "development".to_string();
    config.otp_provider = "mock".to_string();
    config.billing_fraud_step_up_threshold = 90;
    config.billing_fraud_manual_review_threshold = 80;
    config.billing_fraud_block_threshold = 70;
    let err = validate_config_urls_and_secrets(&config).expect_err("fraud thresholds");
    assert!(err.contains("step_up"));
}
