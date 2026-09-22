use std::{ffi::OsString, sync::Mutex};

use super::BillingConfig;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TEST_VARS: &[&str] = &[
    "NVBES_BILLING_BIND_ADDR",
    "NVBES_BILLING_DATABASE_URL",
    "NVBES_STRIPE_SECRET_KEY",
    "NVBES_STRIPE_WEBHOOK_SECRET",
    "NVBES_STRIPE_API_BASE_URL",
    "NVBES_IDENTITY_PUBLIC_KEY_PEM",
    "NVBES_BILLING_METRICS_TOKEN",
    "NVBES_BILLING_OPERATOR_TOKEN",
    "NVBES_APP_URL",
    "NVBES_EMAIL_GRPC_ENDPOINT",
    "NVBES_BILLING_EMAIL_TOKEN",
    "NVBES_EMAIL_INTERNAL_TOKEN",
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

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn loads_development_defaults() {
    let _lock = env_lock();
    let _guard = EnvGuard::isolated();
    let cfg = BillingConfig::from_env().expect("defaults must load");
    assert_eq!(cfg.bind_addr.to_string(), "0.0.0.0:8080");
    assert!(cfg.stripe_secret_key.starts_with("sk_test_"));
    assert_eq!(cfg.app_url, "https://nvbes.test");
    assert!(cfg.email_token.is_none());
}

#[test]
fn loads_explicit_env_overrides() {
    let _lock = env_lock();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_BIND_ADDR", "127.0.0.1:9090");
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        "postgres://billing.local/nvbes",
    );
    guard.set("NVBES_STRIPE_SECRET_KEY", "sk_test_override");
    guard.set("NVBES_STRIPE_WEBHOOK_SECRET", "whsec_override");
    guard.set("NVBES_STRIPE_API_BASE_URL", "https://stripe.test");
    guard.set("NVBES_BILLING_METRICS_TOKEN", "metrics-token");
    guard.set("NVBES_BILLING_OPERATOR_TOKEN", "operator-token");
    guard.set("NVBES_APP_URL", "https://app.example");
    guard.set("NVBES_EMAIL_GRPC_ENDPOINT", "http://email:50051");
    guard.set("NVBES_EMAIL_INTERNAL_TOKEN", "email-internal");

    let cfg = BillingConfig::from_env().expect("overrides must load");
    assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:9090");
    assert_eq!(cfg.database_url, "postgres://billing.local/nvbes");
    assert_eq!(cfg.stripe_secret_key, "sk_test_override");
    assert_eq!(cfg.stripe_webhook_secret, "whsec_override");
    assert_eq!(cfg.stripe_api_base_url, "https://stripe.test");
    assert_eq!(cfg.metrics_token.as_deref(), Some("metrics-token"));
    assert_eq!(cfg.operator_token.as_deref(), Some("operator-token"));
    assert_eq!(cfg.app_url, "https://app.example");
    assert_eq!(
        cfg.email_grpc_endpoint.as_deref(),
        Some("http://email:50051")
    );
    assert_eq!(cfg.email_token.as_deref(), Some("email-internal"));
}

#[test]
fn prefers_billing_email_token_over_internal() {
    let _lock = env_lock();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_EMAIL_TOKEN", "billing-specific");
    guard.set("NVBES_EMAIL_INTERNAL_TOKEN", "shared-internal");
    let cfg = BillingConfig::from_env().expect("config must load");
    assert_eq!(cfg.email_token.as_deref(), Some("billing-specific"));
}

#[test]
fn rejects_live_stripe_secret_keys() {
    let _lock = env_lock();
    let guard = EnvGuard::isolated();
    let live = format!("{}_{}", "sk_live", "forbidden");
    guard.set("NVBES_STRIPE_SECRET_KEY", &live);
    let err = BillingConfig::from_env().expect_err("live keys must fail");
    assert!(err.to_string().contains("FINOPS"));
}

#[test]
fn rejects_live_stripe_restricted_keys() {
    let _lock = env_lock();
    let guard = EnvGuard::isolated();
    let live = format!("{}_{}", "rk_live", "forbidden");
    guard.set("NVBES_STRIPE_SECRET_KEY", &live);
    assert!(BillingConfig::from_env().is_err());
}

#[test]
fn rejects_invalid_bind_addr() {
    let _lock = env_lock();
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_BIND_ADDR", "not-an-addr");
    assert!(BillingConfig::from_env().is_err());
}
