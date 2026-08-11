use super::{EnterpriseGrpcConfig, normalized};

const VALID_TOKEN: &str = "identity-worker-enterprise-token-32";

#[test]
fn absent_or_blank_endpoint_disables_enterprise_integration() {
    assert!(
        EnterpriseGrpcConfig::from_values("production", None, None)
            .expect("absent")
            .is_none()
    );
    assert!(
        EnterpriseGrpcConfig::from_values("production", Some("  ".to_string()), None)
            .expect("blank")
            .is_none()
    );
    assert_eq!(
        normalized(Some(" value ".to_string())).as_deref(),
        Some("value")
    );
    assert_eq!(normalized(Some(String::new())), None);
}

#[test]
fn production_requires_https() {
    let error = EnterpriseGrpcConfig::from_values(
        "production",
        Some("http://enterprise:3031".to_string()),
        Some(VALID_TOKEN.to_string()),
    )
    .err()
    .expect("plaintext endpoint should fail");
    assert!(error.to_string().contains("must use https"));
}

#[test]
fn configured_endpoint_requires_a_present_strong_metadata_safe_token() {
    for token in [
        None,
        Some("short".to_string()),
        Some(format!("{}\n{}", "x".repeat(16), "x".repeat(16))),
    ] {
        let error = EnterpriseGrpcConfig::from_values(
            "development",
            Some("http://127.0.0.1:3031".to_string()),
            token,
        )
        .err()
        .expect("invalid token");
        assert!(
            error.to_string().contains("required")
                || error.to_string().contains("at least 32")
                || error.to_string().contains("metadata-safe")
        );
    }
}

#[test]
fn development_configuration_authorizes_requests() {
    let config = EnterpriseGrpcConfig::from_values(
        "development",
        Some("http://127.0.0.1:3031".to_string()),
        Some(VALID_TOKEN.to_string()),
    )
    .expect("configuration")
    .expect("enabled");

    let request = config.authorize("payload");

    assert_eq!(request.into_inner(), "payload");
    let request = config.authorize(());
    assert_eq!(
        request
            .metadata()
            .get("authorization")
            .expect("authorization"),
        format!("Bearer {VALID_TOKEN}").as_str()
    );
}

#[test]
fn endpoint_must_be_a_valid_uri() {
    let error = EnterpriseGrpcConfig::from_values(
        "development",
        Some("not a uri".to_string()),
        Some(VALID_TOKEN.to_string()),
    )
    .err()
    .expect("invalid URI");
    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn connect_surfaces_an_unreachable_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let endpoint = format!("http://{}", listener.local_addr().expect("address"));
    drop(listener);
    let config = EnterpriseGrpcConfig::from_values(
        "development",
        Some(endpoint),
        Some(VALID_TOKEN.to_string()),
    )
    .expect("configuration")
    .expect("enabled");
    assert!(config.connect().await.is_err());
}
