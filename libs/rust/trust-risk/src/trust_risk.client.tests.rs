use std::time::Duration;

use super::{TrustRiskClientConfig, TrustRiskClientError, map_status};

const TOKEN: &str = "trust-risk-internal-token-at-least-32-characters";

#[test]
fn production_requires_https() {
    let error = TrustRiskClientConfig::from_values(
        "production",
        "http://risk:3050".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(100),
    )
    .unwrap_err();

    assert!(matches!(error, TrustRiskClientError::Configuration(_)));
}

#[test]
fn configuration_requires_strong_metadata_safe_token_and_timeout() {
    for (token, timeout) in [
        ("short", Duration::from_millis(100)),
        (TOKEN, Duration::ZERO),
        (
            "trust-risk-token-at-least-32-characters\nunsafe",
            Duration::from_millis(100),
        ),
    ] {
        assert!(
            TrustRiskClientConfig::from_values(
                "development",
                "http://127.0.0.1:3050".to_string(),
                token.to_string(),
                timeout,
            )
            .is_err()
        );
    }
}

#[test]
fn development_accepts_authenticated_loopback_and_keeps_separate_timeouts() {
    let config = TrustRiskClientConfig::from_values(
        "development",
        "http://127.0.0.1:3050".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(100),
    )
    .unwrap();

    assert_eq!(config.assessment_timeout, Duration::from_millis(100));
    assert_eq!(config.durable_timeout, Duration::from_secs(3));
    assert_eq!(
        config.authorization.to_str().unwrap(),
        format!("Bearer {TOKEN}")
    );
}

#[test]
fn remote_statuses_map_to_stable_errors() {
    assert!(matches!(
        map_status(tonic::Status::already_exists("sensitive remote message")),
        TrustRiskClientError::Conflict
    ));
    assert!(matches!(
        map_status(tonic::Status::failed_precondition("hidden")),
        TrustRiskClientError::Conflict
    ));
    assert!(matches!(
        map_status(tonic::Status::permission_denied("hidden")),
        TrustRiskClientError::Unauthorized
    ));
    assert!(matches!(
        map_status(tonic::Status::unauthenticated("hidden")),
        TrustRiskClientError::Unauthorized
    ));
    assert!(matches!(
        map_status(tonic::Status::deadline_exceeded("hidden")),
        TrustRiskClientError::Unavailable
    ));
    assert!(matches!(
        map_status(tonic::Status::unavailable("hidden")),
        TrustRiskClientError::Unavailable
    ));
    assert!(matches!(
        map_status(tonic::Status::resource_exhausted("hidden")),
        TrustRiskClientError::Unavailable
    ));
    assert!(matches!(
        map_status(tonic::Status::invalid_argument("hidden")),
        TrustRiskClientError::Invalid
    ));
    assert!(matches!(
        map_status(tonic::Status::internal("hidden")),
        TrustRiskClientError::Protocol
    ));
}

#[test]
fn invalid_endpoint_uri_is_rejected() {
    let error = TrustRiskClientConfig::from_values(
        "development",
        "not a uri".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(100),
    )
    .unwrap_err();
    assert!(matches!(error, TrustRiskClientError::Configuration(_)));
}
