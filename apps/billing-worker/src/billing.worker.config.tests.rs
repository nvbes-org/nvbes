use std::{ffi::OsString, sync::Mutex};

use super::{BillingWorkerConfig, DispatchMode};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_BILLING_DATABASE_URL",
    "DATABASE_URL",
    "NVBES_BILLING_HTTP_BIND_ADDR",
    "NVBES_BILLING_BIND_ADDR",
    "NVBES_APP_URL",
    "NVBES_EMAIL_GRPC_ENDPOINT",
    "NVBES_EMAIL_PRODUCER_TOKEN",
    "NVBES_EMAIL_TOKEN",
    "NVBES_BILLING_METRICS_TOKEN",
    "NVBES_BILLING_GRPC_ENDPOINT",
    "NVBES_BILLING_GRPC_AUTH_TOKEN",
    "NVBES_BILLING_QUEUE_ENDPOINT",
    "NVBES_BILLING_QUEUE_URL",
    "NVBES_BILLING_QUEUE_REGION",
    "NVBES_BILLING_QUEUE_ACCESS_KEY",
    "NVBES_BILLING_QUEUE_SECRET_KEY",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
    "STRIPE_SECRET_KEY",
    "NVBES_STRIPE_SECRET_KEY",
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
fn database_url_is_strictly_required() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard = EnvGuard::isolated();
    let res = BillingWorkerConfig::from_env();
    assert!(res.is_err());
}

#[test]
fn loads_default_in_memory_config() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_DATABASE_URL", "postgres://localhost/test_db");
    let cfg = BillingWorkerConfig::from_env().expect("config should load");
    assert_eq!(cfg.environment, "development");
    assert_eq!(cfg.http_bind_addr.to_string(), "0.0.0.0:8080");
    assert!(matches!(cfg.dispatch_mode, DispatchMode::InMemory));
}

#[test]
fn strictly_rejects_live_stripe_keys() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_DATABASE_URL", "postgres://localhost/test_db");
    let test_live_key = format!("{}_{}", "sk_live", "forbidden_sample_token");
    guard.set("NVBES_STRIPE_SECRET_KEY", test_live_key);
    let res = BillingWorkerConfig::from_env();
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("CRITICAL: Live Stripe credentials are fundamentally prohibited")
    );
}

#[test]
fn parses_scaleway_queue_configuration() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_DATABASE_URL", "postgres://localhost/test_db");
    guard.set(
        "NVBES_BILLING_QUEUE_ENDPOINT",
        "https://sqs.mnq.fr-par.scaleway.com",
    );
    guard.set(
        "NVBES_BILLING_QUEUE_URL",
        "https://sqs.mnq.fr-par.scaleway.com/123/billing-dispatch",
    );
    guard.set("NVBES_BILLING_QUEUE_REGION", "fr-par");
    guard.set("NVBES_BILLING_QUEUE_ACCESS_KEY", "SCWXXXXXXXXXXXXXXXXX");
    guard.set(
        "NVBES_BILLING_QUEUE_SECRET_KEY",
        "11111111-2222-3333-4444-555555555555",
    );

    let cfg = BillingWorkerConfig::from_env().expect("config should load");
    match cfg.dispatch_mode {
        DispatchMode::Scaleway(sqs) => {
            assert_eq!(sqs.endpoint, "https://sqs.mnq.fr-par.scaleway.com");
            assert_eq!(sqs.region, "fr-par");
        }
        _ => panic!("expected Scaleway dispatch mode"),
    }
}

#[test]
fn falls_back_to_database_url_when_billing_url_missing() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("DATABASE_URL", "postgres://localhost/fallback_db");
    let cfg = BillingWorkerConfig::from_env().expect("config should load");
    assert_eq!(cfg.database_url, "postgres://localhost/fallback_db");
}

#[test]
fn rejects_invalid_http_bind_address() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_DATABASE_URL", "postgres://localhost/test_db");
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", "not-a-socket-addr");
    let err = BillingWorkerConfig::from_env().expect_err("invalid bind must fail");
    assert!(
        err.to_string()
            .contains("invalid NVBES_BILLING_HTTP_BIND_ADDR")
    );
}

#[test]
fn loads_optional_integrations_from_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "staging");
    guard.set("NVBES_BILLING_DATABASE_URL", "postgres://localhost/test_db");
    guard.set("NVBES_BILLING_BIND_ADDR", "127.0.0.1:9090");
    guard.set("NVBES_APP_URL", "https://app.example.test");
    guard.set("NVBES_EMAIL_GRPC_ENDPOINT", "http://email:50051");
    guard.set("NVBES_EMAIL_PRODUCER_TOKEN", "email-token");
    guard.set("NVBES_BILLING_GRPC_ENDPOINT", "http://billing:50052");
    guard.set("NVBES_BILLING_GRPC_AUTH_TOKEN", "billing-token");
    guard.set("NVBES_BILLING_METRICS_TOKEN", "metrics-token");
    guard.set("NVBES_OTLP_ENDPOINT", "http://otel:4317");
    guard.set("NVBES_OTLP_AUTHORIZATION_HEADER", "Bearer otel");

    let cfg = BillingWorkerConfig::from_env().expect("config should load");
    assert_eq!(cfg.environment, "staging");
    assert_eq!(cfg.http_bind_addr.to_string(), "127.0.0.1:9090");
    assert_eq!(cfg.app_url, "https://app.example.test");
    assert_eq!(
        cfg.email_grpc_endpoint.as_deref(),
        Some("http://email:50051")
    );
    assert_eq!(cfg.email_token.as_deref(), Some("email-token"));
    assert_eq!(
        cfg.billing_grpc_endpoint.as_deref(),
        Some("http://billing:50052")
    );
    assert_eq!(cfg.billing_grpc_token.as_deref(), Some("billing-token"));
    assert_eq!(cfg.metrics_token.as_deref(), Some("metrics-token"));
    assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://otel:4317"));
    assert_eq!(
        cfg.otlp_authorization_header.as_deref(),
        Some("Bearer otel")
    );
}

#[test]
fn strictly_rejects_legacy_live_stripe_secret_key() {
    let _lock = ENV_LOCK.lock().unwrap();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_DATABASE_URL", "postgres://localhost/test_db");
    let test_live_key = format!("{}_{}", "sk_live", "legacy_forbidden_sample");
    guard.set("STRIPE_SECRET_KEY", test_live_key);
    let err = BillingWorkerConfig::from_env().expect_err("live stripe key must fail");
    assert!(
        err.to_string()
            .contains("CRITICAL: Live Stripe credentials")
    );
}
