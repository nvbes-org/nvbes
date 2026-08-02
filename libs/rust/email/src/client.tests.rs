use std::time::Duration;

use super::{EmailClientConfig, EmailClientError};

const VALID_TOKEN: &str = "email-internal-token-at-least-32-characters";

#[test]
fn production_requires_https() {
    let error = EmailClientConfig::from_values(
        "production",
        "http://email-worker:3041".to_string(),
        VALID_TOKEN.to_string(),
        Duration::from_secs(3),
    )
    .err()
    .expect("plaintext endpoint must fail");

    assert!(matches!(error, EmailClientError::Configuration(_)));
    assert!(error.to_string().contains("must use https"));
}

#[test]
fn development_allows_authenticated_loopback_http() {
    let config = EmailClientConfig::from_values(
        "development",
        "http://127.0.0.1:3041".to_string(),
        VALID_TOKEN.to_string(),
        Duration::from_secs(3),
    );

    assert!(config.is_ok());
}

#[test]
fn weak_internal_token_is_rejected() {
    let error = EmailClientConfig::from_values(
        "development",
        "http://127.0.0.1:3041".to_string(),
        "short".to_string(),
        Duration::from_secs(3),
    )
    .err()
    .expect("weak token must fail");

    assert!(error.to_string().contains("at least 32"));
}
