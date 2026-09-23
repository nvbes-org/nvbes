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

fn development_base_config() -> AppConfig {
    AppConfig {
        environment: "development".to_string(),
        web_base_url: "http://localhost:5173".to_string(),
        api_base_url: "http://localhost:4000".to_string(),
        billing_default_success_url: "http://localhost:5173/billing/success".to_string(),
        billing_default_cancel_url: "http://localhost:5173/billing/cancel".to_string(),
        billing_default_portal_return_url: "http://localhost:5173/billing".to_string(),
        webauthn_rp_origin: "http://localhost:3001".to_string(),
        webauthn_rp_id: "localhost".to_string(),
        stripe_api_base_url: "https://api.stripe.com".to_string(),
        mollie_api_base_url: "https://api.mollie.com".to_string(),
        twilio_api_base_url: "https://verify.twilio.com".to_string(),
        database_url: "postgres://postgres:postgres@localhost:5432/nvbes".to_string(),
        jwt_secret: "default-secret-change-me".to_string(),
        otp_provider: "mock".to_string(),
        profiling_sample_rate_hz: 100,
        auth_session_idle_ttl_minutes: 30,
        auth_verification_resend_cooldown_seconds: 60,
        auth_unverified_account_ttl_days: 7,
        auth_factor_encryption_key_version: 1,
        ip_intelligence_timeout_secs: 2,
        ip_intelligence_cache_ttl_hours: 24,
        ..AppConfig::default()
    }
}

#[test]
fn validate_config_mtls_rejects_empty_tls_paths() {
    let cases = [
        ("cert", "NVBES_TLS_CERT_PATH"),
        ("key", "NVBES_TLS_KEY_PATH"),
        ("ca", "NVBES_TLS_CLIENT_CA_PATH"),
    ];
    for (which, expected) in cases {
        let mut config = mtls_config_with_paths();
        match which {
            "cert" => config.tls_cert_path = Some(String::new()),
            "key" => config.tls_key_path = Some(String::new()),
            "ca" => config.tls_client_ca_path = Some(String::new()),
            _ => unreachable!(),
        }
        let err = validate_config_urls_and_secrets(&config).expect_err("empty mtls path");
        assert!(err.contains(expected), "case {which}: {err}");
    }
}

#[test]
fn validate_config_twilio_verify_requires_each_credential() {
    let cases = [
        (
            None,
            Some("token".into()),
            Some("VA123".into()),
            "NVBES_TWILIO_ACCOUNT_SID",
        ),
        (
            Some("AC123".into()),
            None,
            Some("VA123".into()),
            "NVBES_TWILIO_AUTH_TOKEN",
        ),
        (
            Some("AC123".into()),
            Some("token".into()),
            None,
            "NVBES_TWILIO_VERIFY_SERVICE_SID",
        ),
    ];
    for (sid, token, service, expected) in cases {
        let mut config = development_base_config();
        config.otp_provider = "twilio_verify".to_string();
        config.twilio_account_sid = sid;
        config.twilio_auth_token = token;
        config.twilio_verify_service_sid = service;
        let err = validate_config_urls_and_secrets(&config).expect_err("twilio credential");
        assert!(err.contains(expected), "expected {expected} in {err}");
    }
}

#[test]
fn validate_config_accepts_complete_twilio_verify_in_development() {
    let mut config = development_base_config();
    config.otp_provider = "twilio_verify".to_string();
    config.twilio_account_sid = Some("AC123".into());
    config.twilio_auth_token = Some("token".into());
    config.twilio_verify_service_sid = Some("VA123".into());
    validate_config_urls_and_secrets(&config).expect("complete twilio config");
}

#[test]
fn validate_config_rejects_fraud_block_above_100() {
    let mut config = development_base_config();
    config.billing_fraud_step_up_threshold = 10;
    config.billing_fraud_manual_review_threshold = 20;
    config.billing_fraud_block_threshold = 101;
    let err = validate_config_urls_and_secrets(&config).expect_err("block > 100");
    assert!(err.contains("block <= 100"));
}

#[test]
fn validate_config_rejects_manual_review_above_block() {
    let mut config = development_base_config();
    config.billing_fraud_step_up_threshold = 10;
    config.billing_fraud_manual_review_threshold = 90;
    config.billing_fraud_block_threshold = 80;
    let err = validate_config_urls_and_secrets(&config).expect_err("manual > block");
    assert!(err.contains("step_up"));
}

#[test]
fn validate_config_accepts_mtls_with_all_paths_in_development() {
    let mut config = development_base_config();
    config.mtls_enabled = true;
    config.tls_cert_path = Some("/tmp/cert.pem".into());
    config.tls_key_path = Some("/tmp/key.pem".into());
    config.tls_client_ca_path = Some("/tmp/ca.pem".into());
    validate_config_urls_and_secrets(&config).expect("mtls with paths");
}
