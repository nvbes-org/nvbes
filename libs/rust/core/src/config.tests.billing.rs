use super::AppConfig;
use super::validation::validate_config_urls_and_secrets;

#[test]
fn billing_database_url_prefers_dedicated_url() {
    let config = AppConfig {
        database_url: "postgres://postgres:postgres@localhost:5432/nvbes".to_string(),
        billing_database_url: "postgres://postgres:postgres@localhost:5432/nvbes_billing"
            .to_string(),
        ..AppConfig::default()
    };

    assert_eq!(
        config.billing_database_url(),
        "postgres://postgres:postgres@localhost:5432/nvbes_billing"
    );
}

#[test]
fn billing_database_url_falls_back_for_hand_built_test_configs() {
    let config = AppConfig {
        database_url: "postgres://postgres:postgres@localhost:5432/nvbes".to_string(),
        ..AppConfig::default()
    };

    assert_eq!(
        config.billing_database_url(),
        "postgres://postgres:postgres@localhost:5432/nvbes"
    );
}

#[test]
fn validate_config_requires_stripe_secret_when_external_provider_fallback_is_enabled() {
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
        otp_provider: "mock".to_string(),
        twilio_api_base_url: "https://verify.twilio.com".to_string(),
        billing_external_provider_fallback_enabled: true,
        stripe_secret_key: None,
        ..AppConfig::default()
    };

    let error = validate_config_urls_and_secrets(&config)
        .expect_err("external provider fallback must require Stripe credentials");

    assert!(error.contains("NVBES_STRIPE_SECRET_KEY"));
}

#[test]
fn validate_config_requires_ordered_billing_fraud_thresholds() {
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
        otp_provider: "mock".to_string(),
        twilio_api_base_url: "https://verify.twilio.com".to_string(),
        billing_fraud_step_up_threshold: 80,
        billing_fraud_manual_review_threshold: 75,
        billing_fraud_block_threshold: 90,
        ..AppConfig::default()
    };

    let error =
        validate_config_urls_and_secrets(&config).expect_err("fraud thresholds must be ordered");

    assert!(error.contains("NVBES_BILLING_FRAUD"));
}
