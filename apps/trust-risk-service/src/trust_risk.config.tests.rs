use super::{
    ConfigError,
    observability::{ObservabilityConfig, parse_sample_rate, validate},
    parse_operators, parse_producers,
};

const TOKEN: &str = "producer-token-with-at-least-32-characters";

#[test]
fn parses_explicit_producer_permissions() {
    let value = format!(
        r#"[{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":["identity.","automation."],"can_assess":true}}]"#
    );
    let policies = parse_producers(&value).unwrap();
    let policy = &policies["identity-service"];
    assert!(policy.permits_signal("identity.login"));
    assert!(!policy.permits_signal("payment.checkout"));
    assert!(policy.can_assess);
    assert!(!policy.can_label);
}

#[test]
fn rejects_duplicate_policies_and_weak_tokens() {
    let duplicate = format!(
        r#"[{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":[]}},{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":[]}}]"#
    );
    assert_eq!(
        parse_producers(&duplicate).unwrap_err(),
        ConfigError::DuplicatePolicy
    );
    assert_eq!(
        parse_operators(r#"[{"actor":"ada","token":"short","permissions":[]}]"#).unwrap_err(),
        ConfigError::WeakToken
    );
}

#[test]
fn operator_permissions_are_explicit() {
    let value = format!(
        r#"[{{"actor":"operator:ada","token":"{TOKEN}","permissions":["evaluation:read"]}}]"#
    );
    let policies = parse_operators(&value).unwrap();
    assert!(policies["operator:ada"].permits("evaluation:read"));
    assert!(!policies["operator:ada"].permits("rules:write"));
}

#[test]
fn observability_sample_rates_are_bounded() {
    assert_eq!(parse_sample_rate(None).unwrap(), 0.0);
    assert_eq!(parse_sample_rate(Some("0.1")).unwrap(), 0.1);
    assert!(parse_sample_rate(Some("-0.1")).is_err());
    assert!(parse_sample_rate(Some("1.1")).is_err());
    assert!(parse_sample_rate(Some("invalid")).is_err());
}

#[test]
fn production_requires_sentry_and_authenticated_https_otlp() {
    let valid = ObservabilityConfig {
        sentry_dsn: Some("https://public@example.ingest.sentry.io/42".to_string()),
        sentry_traces_sample_rate: 0.1,
        otlp_endpoint: Some("https://otlp-gateway.example.grafana.net:443".to_string()),
        otlp_authorization_header: Some("Basic dXNlcjp0b2tlbg==".to_string()),
    };
    assert!(validate("production", &valid).is_ok());

    let mut missing_sentry = valid.clone();
    missing_sentry.sentry_dsn = None;
    assert!(validate("production", &missing_sentry).is_err());

    let mut insecure_otlp = valid.clone();
    insecure_otlp.otlp_endpoint = Some("http://collector:4317".to_string());
    assert!(validate("production", &insecure_otlp).is_err());

    let mut missing_auth = valid;
    missing_auth.otlp_authorization_header = None;
    assert!(validate("production", &missing_auth).is_err());
}
