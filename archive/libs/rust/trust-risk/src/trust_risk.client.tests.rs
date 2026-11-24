use std::time::Duration;

use super::{MIN_TOKEN_LENGTH, TrustRiskClientConfig, TrustRiskClientError, map_status};

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
        (
            &"x".repeat(MIN_TOKEN_LENGTH - 1),
            Duration::from_millis(100),
        ),
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

    TrustRiskClientConfig::from_values(
        "development",
        "http://127.0.0.1:3050".to_string(),
        "x".repeat(MIN_TOKEN_LENGTH),
        Duration::from_millis(100),
    )
    .expect("exact minimum token length must be accepted");
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

#[test]
fn production_accepts_https_and_configures_tls() {
    let config = TrustRiskClientConfig::from_values(
        "production",
        "https://trust-risk.example.invalid:443".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(100),
    )
    .expect("https is required and accepted in production");
    assert_eq!(config.assessment_timeout, Duration::from_millis(100));
    assert_eq!(
        config.authorization.to_str().unwrap(),
        format!("Bearer {TOKEN}")
    );
}

#[test]
fn from_env_requires_endpoint_and_token() {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap();

    let old_endpoint = std::env::var("NVBES_TRUST_RISK_GRPC_ENDPOINT").ok();
    let old_token = std::env::var("NVBES_TRUST_RISK_GRPC_AUTH_TOKEN").ok();

    // SAFETY: static mutex guard serializes test environment variable mutations.
    unsafe {
        std::env::remove_var("NVBES_TRUST_RISK_GRPC_ENDPOINT");
        std::env::remove_var("NVBES_TRUST_RISK_GRPC_AUTH_TOKEN");
    }
    let err = TrustRiskClientConfig::from_env("development").unwrap_err();
    assert!(matches!(err, TrustRiskClientError::Configuration(_)));

    // SAFETY: static mutex guard serializes test environment variable mutations.
    unsafe {
        std::env::set_var("NVBES_TRUST_RISK_GRPC_ENDPOINT", "http://127.0.0.1:3050");
    }
    let err2 = TrustRiskClientConfig::from_env("development").unwrap_err();
    assert!(matches!(err2, TrustRiskClientError::Configuration(_)));

    // SAFETY: static mutex guard serializes test environment variable mutations.
    unsafe {
        std::env::set_var("NVBES_TRUST_RISK_GRPC_AUTH_TOKEN", TOKEN);
    }
    let config = TrustRiskClientConfig::from_env("development").unwrap();
    assert_eq!(config.assessment_timeout, Duration::from_millis(100));
    assert_eq!(
        config.authorization.to_str().unwrap(),
        format!("Bearer {TOKEN}")
    );

    // SAFETY: static mutex guard serializes test environment variable mutations.
    unsafe {
        match old_endpoint {
            Some(val) => std::env::set_var("NVBES_TRUST_RISK_GRPC_ENDPOINT", val),
            None => std::env::remove_var("NVBES_TRUST_RISK_GRPC_ENDPOINT"),
        }
        match old_token {
            Some(val) => std::env::set_var("NVBES_TRUST_RISK_GRPC_AUTH_TOKEN", val),
            None => std::env::remove_var("NVBES_TRUST_RISK_GRPC_AUTH_TOKEN"),
        }
    }
}

#[tokio::test]
async fn connect_to_unreachable_endpoint_returns_unavailable() {
    let config = TrustRiskClientConfig::from_values(
        "development",
        "http://127.0.0.1:9".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(50),
    )
    .unwrap();
    let res = super::TrustRiskClient::connect(config).await;
    assert!(matches!(res, Err(TrustRiskClientError::Unavailable)));
}

#[tokio::test]
async fn lazy_client_request_construction_and_unavailable_network_call() {
    let config = TrustRiskClientConfig::from_values(
        "development",
        "http://127.0.0.1:9".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(50),
    )
    .unwrap();
    let client = super::TrustRiskClient::connect_lazy(config);
    let req = client.request("test_payload", Duration::from_millis(100));
    assert_eq!(
        req.metadata().get("authorization").unwrap(),
        &format!("Bearer {TOKEN}")
    );

    let err = client
        .assess_risk(crate::proto::nvbes::trust_risk::v1::AssessRiskRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(err, TrustRiskClientError::Unavailable));

    let err2 = client
        .submit_signals(crate::proto::nvbes::trust_risk::v1::SubmitSignalsRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(err2, TrustRiskClientError::Unavailable));

    let err3 = client
        .submit_labels(crate::proto::nvbes::trust_risk::v1::SubmitLabelsRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(err3, TrustRiskClientError::Unavailable));
}
