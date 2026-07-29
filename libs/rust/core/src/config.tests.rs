use super::AppConfig;
use super::env::env_or_default;
use super::validation::validate_config_urls_and_secrets;
use super::validation::{
    validate_auth_factor_encryption, validate_grafana_export_path, validate_ip_intelligence,
    validate_jwt_secret, validate_observability_internal_token, validate_positive_integer,
    validate_product_analytics, validate_profiling, validate_profiling_endpoint,
    validate_public_url, validate_request_e2ee, validate_sentry, validate_webauthn_rp_id,
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
fn validate_sentry_rejects_invalid_trace_sample_rate() {
    let config = AppConfig {
        sentry_traces_sample_rate: 1.5,
        ..AppConfig::default()
    };

    let error = validate_sentry(&config).expect_err("sample rate above 1 must be rejected");

    assert!(error.contains("SENTRY_TRACES_SAMPLE_RATE"));
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
fn validate_product_analytics_requires_token_and_salt_when_enabled() {
    let config = AppConfig {
        product_analytics_enabled: true,
        ..AppConfig::default()
    };

    let error = validate_product_analytics(&config, false)
        .expect_err("enabled product analytics must require a token");

    assert!(error.contains("NVBES_PRODUCT_ANALYTICS_TOKEN"));
}

#[test]
fn validate_product_analytics_requires_strong_salt_in_strict_mode() {
    let config = AppConfig {
        product_analytics_enabled: true,
        product_analytics_token: Some("analytics_test".to_string()),
        analytics_id_salt: Some("short".to_string()),
        ..AppConfig::default()
    };

    let error = validate_product_analytics(&config, true)
        .expect_err("strict mode must reject weak analytics salt");

    assert!(error.contains("NVBES_ANALYTICS_ID_SALT"));
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

#[test]
fn validate_auth_factor_encryption_requires_a_production_key() {
    let config = AppConfig {
        environment: "production".to_string(),
        ..AppConfig::default()
    };

    let error = validate_auth_factor_encryption(&config, true)
        .expect_err("production must require factor encryption");

    assert!(error.contains("NVBES_AUTH_FACTOR_ENCRYPTION_KEY"));
}

#[test]
fn validate_auth_factor_encryption_rejects_short_decoded_keys() {
    let config = AppConfig {
        auth_factor_encryption_key: Some("c2hvcnQ=".to_string()),
        auth_factor_encryption_key_version: 1,
        ..AppConfig::default()
    };

    let error = validate_auth_factor_encryption(&config, false)
        .expect_err("short factor encryption keys must be rejected");

    assert!(error.contains("32 bytes"));
}

#[test]
fn validate_config_rejects_mock_otp_outside_development() {
    let config = AppConfig {
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
        jwt_secret: "a-very-long-production-jwt-secret-value".to_string(),
        observability_internal_token: Some("a-very-long-observability-token-value".to_string()),
        otp_provider: "mock".to_string(),
        ..AppConfig::default()
    };

    let error = validate_config_urls_and_secrets(&config)
        .expect_err("strict mode must reject mock OTP provider");

    assert!(error.contains("NVBES_OTP_PROVIDER=mock"));
}

#[test]
fn validate_config_requires_twilio_verify_credentials() {
    let config = AppConfig {
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
        database_url: "postgres://postgres:postgres@localhost:5432/nvbes".to_string(),
        jwt_secret: "default-secret-change-me".to_string(),
        otp_provider: "twilio_verify".to_string(),
        twilio_api_base_url: "https://verify.twilio.com".to_string(),
        ..AppConfig::default()
    };

    let error = validate_config_urls_and_secrets(&config)
        .expect_err("twilio provider must require credentials");

    assert!(error.contains("NVBES_TWILIO_ACCOUNT_SID"));
}

#[test]
fn validate_ip_intelligence_rejects_bad_provider_specs() {
    let missing_placeholder = AppConfig {
        ip_intelligence_provider_specs: vec![
            "ipinfo|https://example.test/no-placeholder".to_string(),
        ],
        ip_intelligence_timeout_secs: 2,
        ip_intelligence_cache_ttl_hours: 24,
        ..AppConfig::default()
    };
    let error = validate_ip_intelligence(&missing_placeholder, false)
        .expect_err("provider URL must include ip placeholder");
    assert!(error.contains("{ip}"));

    let insecure = AppConfig {
        ip_intelligence_provider_specs: vec!["ipinfo|http://example.test/{ip}".to_string()],
        ip_intelligence_timeout_secs: 2,
        ip_intelligence_cache_ttl_hours: 24,
        ..AppConfig::default()
    };
    let error =
        validate_ip_intelligence(&insecure, true).expect_err("strict mode must require HTTPS");
    assert!(error.contains("HTTPS"));
}
