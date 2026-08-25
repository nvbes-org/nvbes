use std::time::Duration;

use super::{EmailClientConfig, EmailClientError};
use crate::environment_test_support::ScopedEnvironment;

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

#[test]
fn malformed_endpoint_timeout_and_metadata_are_rejected() {
    for (endpoint, token, timeout, expected) in [
        (
            "not a URI",
            VALID_TOKEN,
            Duration::from_secs(3),
            "valid URI",
        ),
        (
            "http://127.0.0.1:3041",
            VALID_TOKEN,
            Duration::ZERO,
            "positive",
        ),
        (
            "http://127.0.0.1:3041",
            "email-internal-token-over-32-characters\nunsafe",
            Duration::from_secs(3),
            "metadata-safe",
        ),
    ] {
        let error = EmailClientConfig::from_values(
            "development",
            endpoint.to_string(),
            token.to_string(),
            timeout,
        )
        .err()
        .expect("configuration must fail");
        assert!(error.to_string().contains(expected));
    }
}

#[test]
fn https_endpoint_is_configured_outside_development() {
    EmailClientConfig::from_values(
        "production",
        "https://email.example.com".to_string(),
        VALID_TOKEN.to_string(),
        Duration::from_secs(3),
    )
    .expect("HTTPS endpoint should be accepted");
}

#[test]
fn environment_configuration_requires_both_values() {
    {
        let _environment = ScopedEnvironment::replace(&[
            ("NVBES_EMAIL_GRPC_ENDPOINT", None),
            ("NVBES_EMAIL_GRPC_AUTH_TOKEN", None),
        ]);
        assert!(
            EmailClientConfig::from_env("development")
                .err()
                .expect("missing endpoint")
                .to_string()
                .contains("ENDPOINT")
        );
    }
    {
        let _environment = ScopedEnvironment::replace(&[
            ("NVBES_EMAIL_GRPC_ENDPOINT", Some("http://127.0.0.1:3041")),
            ("NVBES_EMAIL_GRPC_AUTH_TOKEN", None),
        ]);
        assert!(
            EmailClientConfig::from_env("development")
                .err()
                .expect("missing token")
                .to_string()
                .contains("AUTH_TOKEN")
        );
    }
    let _environment = ScopedEnvironment::replace(&[
        ("NVBES_EMAIL_GRPC_ENDPOINT", Some("http://127.0.0.1:3041")),
        ("NVBES_EMAIL_GRPC_AUTH_TOKEN", Some(VALID_TOKEN)),
    ]);
    let config = EmailClientConfig::from_env("development").expect("complete environment");
    assert!(config.call_timeout >= Duration::from_secs(15));
}
