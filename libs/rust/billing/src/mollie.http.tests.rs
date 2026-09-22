use reqwest::StatusCode;

use super::{mollie_error, mollie_get_json, mollie_post_json};
use crate::mollie::MollieProviderError;
use nvbes_core::config::AppConfig;

#[test]
fn mollie_error_reads_detail_field() {
    let err = mollie_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        r#"{"detail":"invalid amount"}"#,
    );
    assert!(matches!(
        err,
        MollieProviderError::RequestRejected {
            status: 422,
            message
        } if message == "invalid amount"
    ));
}

#[test]
fn mollie_error_falls_back_to_http_status() {
    let err = mollie_error(StatusCode::INTERNAL_SERVER_ERROR, "plain text");
    assert!(matches!(
        err,
        MollieProviderError::RequestRejected {
            status: 500,
            message
        } if message.contains("500")
    ));
}

#[tokio::test]
async fn mollie_post_json_requires_api_key() {
    let config = AppConfig {
        mollie_api_key: None,
        ..AppConfig::default()
    };
    let err = mollie_post_json(&config, "/v2/payments", serde_json::json!({}))
        .await
        .expect_err("not configured");
    assert!(matches!(err, MollieProviderError::NotConfigured));
}

#[tokio::test]
async fn mollie_get_json_trims_base_url_and_fails_without_network() {
    let config = AppConfig {
        mollie_api_key: Some("test_abc".to_string()),
        mollie_api_base_url: "https://api.mollie.test/".to_string(),
        ..AppConfig::default()
    };
    let err = mollie_get_json(&config, "/v2/payments/tr_test")
        .await
        .expect_err("dns or tls failure");
    assert!(matches!(
        err,
        MollieProviderError::RequestFailed(_) | MollieProviderError::ResponseFailed(_)
    ));
}
