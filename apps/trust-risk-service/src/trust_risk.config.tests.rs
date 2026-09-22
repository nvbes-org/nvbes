use std::{ffi::OsString, sync::Mutex};

use super::{
    ConfigError, TrustRiskConfig, database_url_from_env,
    observability::{ObservabilityConfig, parse_sample_rate, validate},
    parse_operators, parse_producers,
};

const TOKEN: &str = "producer-token-with-at-least-32-characters";

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_TRUST_RISK_DATABASE_URL",
    "NVBES_TRUST_RISK_BIND_ADDR",
    "NVBES_TRUST_RISK_PRODUCER_POLICIES",
    "NVBES_TRUST_RISK_OPERATOR_TOKENS",
    "NVBES_TRUST_RISK_METRICS_TOKEN",
    "NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS",
    "NVBES_TRUST_RISK_EVALUATIONS_RETENTION_DAYS",
    "NVBES_TRUST_RISK_LABELS_RETENTION_DAYS",
    "NVBES_TRUST_RISK_REVIEWS_RETENTION_DAYS",
    "NVBES_TRUST_RISK_AUDIT_RETENTION_DAYS",
    "SENTRY_DSN",
    "NVBES_SENTRY_DSN",
    "SENTRY_TRACES_SAMPLE_RATE",
    "NVBES_SENTRY_TRACES_SAMPLE_RATE",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
];

struct EnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn isolated() -> Self {
        let saved = TEST_VARS
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in TEST_VARS {
            unsafe { std::env::remove_var(name) };
        }
        Self { saved }
    }

    fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe { std::env::set_var(name, value) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, val) in &self.saved {
            match val {
                Some(v) => unsafe { std::env::set_var(name, v) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

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

#[test]
fn from_env_loads_development_defaults() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard = EnvGuard::isolated();
    let cfg = TrustRiskConfig::from_env().expect("development defaults");
    assert_eq!(cfg.environment, "development");
    assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:3050");
    assert!(cfg.producers.contains_key("billing-checkout-fixture"));
    assert!(cfg.operators.contains_key("development-operator"));
    assert_eq!(cfg.retention.signals_days, 30);
    assert_eq!(cfg.retention.audit_days, 730);
}

#[test]
fn from_env_case_table_overrides_retention_and_bind() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "test");
    guard.set("NVBES_TRUST_RISK_BIND_ADDR", "127.0.0.1:4050");
    guard.set("NVBES_TRUST_RISK_DATABASE_URL", "postgres://trust.test/db");
    guard.set("NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS", "14");
    guard.set("NVBES_TRUST_RISK_AUDIT_RETENTION_DAYS", "100");
    guard.set(
        "NVBES_TRUST_RISK_METRICS_TOKEN",
        "metrics-token-with-at-least-32-characters",
    );
    let producer = format!(
        r#"[{{"producer":"identity-service","token":"{TOKEN}","signal_prefixes":["identity."],"can_assess":true}}]"#
    );
    let operator = format!(
        r#"[{{"actor":"operator:ada","token":"{TOKEN}","permissions":["evaluation:read"]}}]"#
    );
    guard.set("NVBES_TRUST_RISK_PRODUCER_POLICIES", &producer);
    guard.set("NVBES_TRUST_RISK_OPERATOR_TOKENS", &operator);

    let cfg = TrustRiskConfig::from_env().expect("overrides");
    assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:4050");
    assert_eq!(cfg.database_url, "postgres://trust.test/db");
    assert_eq!(cfg.retention.signals_days, 14);
    assert_eq!(cfg.retention.audit_days, 100);
    assert!(cfg.producers["identity-service"].can_assess);
}

#[test]
fn from_env_rejects_invalid_bind_and_retention() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_TRUST_RISK_BIND_ADDR", "not-a-socket");
    assert_eq!(
        TrustRiskConfig::from_env().unwrap_err(),
        ConfigError::Invalid("NVBES_TRUST_RISK_BIND_ADDR")
    );
    drop(guard);

    let guard = EnvGuard::isolated();
    guard.set("NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS", "0");
    assert_eq!(
        TrustRiskConfig::from_env().unwrap_err(),
        ConfigError::Invalid("NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS")
    );
}

#[test]
fn production_requires_explicit_database_and_policies() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "production");
    assert_eq!(
        database_url_from_env().unwrap_err(),
        ConfigError::Missing("NVBES_TRUST_RISK_DATABASE_URL")
    );
    assert!(matches!(
        TrustRiskConfig::from_env().unwrap_err(),
        ConfigError::Missing(_)
    ));
}

#[test]
fn rejects_empty_policy_lists_and_bad_identifiers() {
    assert_eq!(
        parse_producers("[]").unwrap_err(),
        ConfigError::Invalid("policy list")
    );
    let bad_id = format!(r#"[{{"producer":"ab","token":"{TOKEN}","signal_prefixes":[]}}]"#);
    assert_eq!(
        parse_producers(&bad_id).unwrap_err(),
        ConfigError::Invalid("policy identifier")
    );
}
