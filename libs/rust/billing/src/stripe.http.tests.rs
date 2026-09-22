use reqwest::StatusCode;

use super::{stripe_error, stripe_post_form};
use crate::stripe::StripeProviderError;
use nvbes_core::config::AppConfig;

#[test]
fn stripe_error_extracts_nested_message() {
    let body = r#"{"error":{"message":"card was declined"}}"#;
    let err = stripe_error(StatusCode::BAD_REQUEST, body);
    assert!(matches!(
        err,
        StripeProviderError::RequestRejected {
            status: 400,
            message
        } if message == "card was declined"
    ));
}

#[test]
fn stripe_error_falls_back_to_status_summary() {
    let err = stripe_error(StatusCode::SERVICE_UNAVAILABLE, "not-json");
    assert!(matches!(
        err,
        StripeProviderError::RequestRejected {
            status: 503,
            message
        } if message.contains("503")
    ));
}

#[tokio::test]
async fn stripe_post_form_rejects_missing_secret() {
    let config = AppConfig {
        stripe_secret_key: None,
        ..AppConfig::default()
    };
    let err = stripe_post_form(&config, "/v1/customers", vec![])
        .await
        .expect_err("not configured");
    assert!(matches!(err, StripeProviderError::NotConfigured));
}

#[tokio::test]
async fn stripe_post_form_rejects_live_secret_keys() {
    let config = AppConfig {
        stripe_secret_key: Some("rk_live_abc".to_string()),
        ..AppConfig::default()
    };
    let err = stripe_post_form(&config, "/v1/customers", vec![])
        .await
        .expect_err("live key");
    assert!(matches!(err, StripeProviderError::LiveKeyRejected));
}

#[tokio::test]
async fn stripe_post_form_surfaces_connection_failures() {
    let config = AppConfig {
        stripe_secret_key: Some("sk_test_local".to_string()),
        stripe_api_base_url: "https://127.0.0.1:1".to_string(),
        ..AppConfig::default()
    };
    let err = stripe_post_form(
        &config,
        "/v1/checkout/sessions",
        vec![("mode".to_string(), "subscription".to_string())],
    )
    .await
    .expect_err("unreachable host");
    assert!(matches!(err, StripeProviderError::RequestFailed(_)));
}
