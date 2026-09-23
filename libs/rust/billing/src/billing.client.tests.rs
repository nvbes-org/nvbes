use std::time::Duration;

use super::{BillingClient, BillingClientConfig, BillingClientError, map_status};
use tonic::{Code, Status};

#[test]
fn billing_client_config_from_values_validates_auth_token_length() {
    let err = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:50051".to_string(),
        "short".to_string(),
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_from_values_rejects_zero_timeout() {
    let err = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:50051".to_string(),
        "01234567890123456789012345678901".to_string(),
        Duration::ZERO,
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_requires_https_outside_development() {
    let err = BillingClientConfig::from_values(
        "production",
        "http://127.0.0.1:50051".to_string(),
        "01234567890123456789012345678901".to_string(),
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_accepts_https_endpoint() {
    BillingClientConfig::from_values(
        "production",
        "https://billing.internal:50051".to_string(),
        "01234567890123456789012345678901".to_string(),
        Duration::from_secs(5),
    )
    .expect("https endpoint should validate");
}

#[test]
fn map_status_classifies_grpc_codes() {
    assert!(matches!(
        map_status(Status::new(Code::Unauthenticated, "")),
        BillingClientError::Unauthorized
    ));
    assert!(matches!(
        map_status(Status::new(Code::PermissionDenied, "")),
        BillingClientError::Unauthorized
    ));
    assert!(matches!(
        map_status(Status::new(Code::Unavailable, "")),
        BillingClientError::Unavailable
    ));
    assert!(matches!(
        map_status(Status::new(Code::DeadlineExceeded, "")),
        BillingClientError::Unavailable
    ));
    assert!(matches!(
        map_status(Status::new(Code::ResourceExhausted, "")),
        BillingClientError::Unavailable
    ));
    assert!(matches!(
        map_status(Status::new(Code::InvalidArgument, "")),
        BillingClientError::Protocol
    ));
}

#[tokio::test]
async fn billing_client_connect_marks_unreachable_endpoints_unavailable() {
    let config = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:1".to_string(),
        "01234567890123456789012345678901".to_string(),
        Duration::from_millis(200),
    )
    .expect("config");
    let err = BillingClient::connect(config).await.unwrap_err();
    assert!(matches!(err, BillingClientError::Unavailable));
}
