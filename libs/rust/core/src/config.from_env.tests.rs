use std::{ffi::OsString, sync::Mutex};

use super::AppConfig;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Variables exercised by from_env case tables. Never touch DATABASE_URL /
/// NVBES_SECURITY_TEST_DATABASE_URL — parallel sqlx tests depend on them.
const TEST_VARS: &[&str] = &[
    "NVBES_ENV",
    "NVBES_APP_NAME",
    "NVBES_API_PORT",
    "NVBES_WEB_BASE_URL",
    "NVBES_API_BASE_URL",
    "NVBES_STAGING_WEB_BASE_URL",
    "NVBES_STAGING_API_BASE_URL",
    "NVBES_ADDITIONAL_CORS_ORIGINS",
    "NVBES_DATABASE_URL",
    "NVBES_BILLING_DATABASE_URL",
    "NVBES_DATABASE_MAX_CONNECTIONS",
    "NVBES_AUTH_SESSION_TTL_HOURS",
    "NVBES_JWT_SECRET",
    "NVBES_SECRET_MANAGER_ENABLED",
    "NVBES_STRIPE_SECRET_KEY",
    "NVBES_STRIPE_WEBHOOK_SECRET",
    "NVBES_STRIPE_API_BASE_URL",
    "NVBES_MOLLIE_API_KEY",
    "NVBES_MOLLIE_ENABLED",
    "NVBES_BILLING_SUCCESS_URL",
    "NVBES_BILLING_CANCEL_URL",
    "NVBES_BILLING_PORTAL_RETURN_URL",
    "NVBES_WEBAUTHN_RP_ID",
    "NVBES_WEBAUTHN_RP_ORIGIN",
    "NVBES_WEBAUTHN_RELATED_ORIGINS",
    "SENTRY_DSN",
    "NVBES_SENTRY_DSN",
    "SENTRY_TRACES_SAMPLE_RATE",
    "NVBES_SENTRY_TRACES_SAMPLE_RATE",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
    "NVBES_POSTHOG_ENABLED",
    "NVBES_PRODUCT_ANALYTICS_ENABLED",
    "NVBES_POSTHOG_PROJECT_TOKEN",
    "NVBES_PRODUCT_ANALYTICS_TOKEN",
    "NVBES_POSTHOG_HOST",
    "NVBES_ANALYTICS_ID_SALT",
    "NVBES_PROFILING_ENABLED",
    "NVBES_PROFILING_ENDPOINT",
    "NVBES_OBSERVABILITY_INTERNAL_TOKEN",
    "NVBES_KMS_ENABLED",
    "NVBES_OTP_PROVIDER",
    "NVBES_TWILIO_VERIFY_SERVICE_SID",
    "STORAGE_ENABLED",
    "STORAGE_BUCKET",
    "STORAGE_ACCESS_KEY",
    "AWS_ACCESS_KEY_ID",
    "STORAGE_SECRET_KEY",
    "AWS_SECRET_ACCESS_KEY",
    "SCAN_ENABLED",
    "SCAN_ENGINE",
    "SCAN_FAIL_OPEN",
    "NVBES_TRUSTED_PROXY_CIDRS",
    "NVBES_IP_INTELLIGENCE_PROVIDERS",
    "NVBES_IP_INTELLIGENCE_TIMEOUT_SECS",
    "NVBES_MAXMIND_GEOLITE_DATABASE_ENABLED",
    "NVBES_MAXMIND_GEOLITE_WEB_ENABLED",
    "NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED",
    "NVBES_MAXMIND_ACCOUNT_ID",
    "NVBES_MAXMIND_LICENSE_KEY",
    "NVBES_LOYALSOLDIER_GEOIP_ENABLED",
    "NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED",
    "NVBES_MTLS_ENABLED",
    "NVBES_MTLS_PORT",
    "NVBES_TLS_CERT_PATH",
    "NVBES_TLS_KEY_PATH",
    "NVBES_TLS_CLIENT_CA_PATH",
    "NVBES_DPOP_ENABLED",
    "NVBES_FAPI_HIGH_ASSURANCE_ENABLED",
    "NVBES_REQUEST_E2EE_ENABLED",
    "NVBES_REQUEST_E2EE_REQUIRED",
    "NVBES_REQUEST_E2EE_KEY_ID",
    "NVBES_REQUEST_E2EE_SECRET",
    "NVBES_SECURITY_CONTACT_EMAIL",
    "NVBES_BILLING_SERVICE_BASE_URL",
    "NVBES_AUTH_POW_ENABLED",
    "NVBES_AUTH_POW_DIFFICULTY",
    "NVBES_BILLING_FRAUD_ENFORCEMENT_ENABLED",
    "NVBES_BILLING_FRAUD_STEP_UP_THRESHOLD",
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

    fn apply(&self, cases: &[(&str, &str)]) {
        for (name, value) in cases {
            self.set(name, value);
        }
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
fn development_defaults_load_without_explicit_env() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _guard = EnvGuard::isolated();
    let cfg = AppConfig::from_env().expect("development defaults");
    assert_eq!(cfg.environment, "development");
    assert_eq!(cfg.api_port, 4000);
    assert_eq!(cfg.app_name, "nvbes Drive");
    assert!(!cfg.secret_manager_enabled);
    assert_eq!(cfg.otp_provider, "mock");
    assert_eq!(cfg.scan_engine, "mock");
    assert!(cfg.scan_fail_open);
    assert!(!cfg.maxmind_geolite_database_enabled);
    assert!(!cfg.loyalsoldier_geoip_enabled);
}

#[test]
fn case_table_overrides_numeric_and_flag_fields() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = EnvGuard::isolated();
    guard.apply(&[
        ("NVBES_ENV", "development"),
        ("NVBES_APP_NAME", "Coverage Drive"),
        ("NVBES_API_PORT", "4100"),
        ("NVBES_OTP_PROVIDER", "mock"),
        ("NVBES_DATABASE_MAX_CONNECTIONS", "7"),
        ("NVBES_AUTH_SESSION_TTL_HOURS", "48"),
        ("NVBES_AUTH_POW_ENABLED", "false"),
        ("NVBES_AUTH_POW_DIFFICULTY", "12"),
        (
            "NVBES_ADDITIONAL_CORS_ORIGINS",
            "https://a.test, https://b.test",
        ),
        ("NVBES_WEBAUTHN_RELATED_ORIGINS", "https://rp.test"),
        ("NVBES_TRUSTED_PROXY_CIDRS", "10.0.0.0/8,192.168.0.0/16"),
        ("NVBES_IP_INTELLIGENCE_PROVIDERS", "ipinfo|https://x/{ip}"),
        ("NVBES_IP_INTELLIGENCE_TIMEOUT_SECS", "5"),
        ("NVBES_MAXMIND_GEOLITE_DATABASE_ENABLED", "true"),
        ("NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED", "1"),
        ("NVBES_MAXMIND_ACCOUNT_ID", "acct"),
        ("NVBES_MAXMIND_LICENSE_KEY", "lic"),
        ("NVBES_LOYALSOLDIER_GEOIP_ENABLED", "true"),
        ("NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED", "true"),
        ("NVBES_POSTHOG_ENABLED", "true"),
        (
            "NVBES_POSTHOG_PROJECT_TOKEN",
            "phc_coverage_token_32chars_min",
        ),
        ("NVBES_ANALYTICS_ID_SALT", "coverage-analytics-salt-32chars"),
        ("NVBES_POSTHOG_HOST", "https://posthog.test"),
        ("NVBES_BILLING_FRAUD_ENFORCEMENT_ENABLED", "true"),
        ("NVBES_BILLING_FRAUD_STEP_UP_THRESHOLD", "55"),
        ("STORAGE_ENABLED", "true"),
        ("STORAGE_BUCKET", "coverage-bucket"),
        ("AWS_ACCESS_KEY_ID", "aws-key"),
        ("AWS_SECRET_ACCESS_KEY", "aws-secret"),
        ("SCAN_ENABLED", "true"),
        ("SCAN_ENGINE", "clamav"),
        ("SCAN_FAIL_OPEN", "false"),
        ("NVBES_REQUEST_E2EE_KEY_ID", "k1"),
        ("NVBES_SECURITY_CONTACT_EMAIL", "sec@example.com"),
        ("NVBES_MTLS_PORT", "4443"),
        ("SENTRY_TRACES_SAMPLE_RATE", "0.25"),
    ]);

    let cfg = AppConfig::from_env().expect("overrides must load");
    assert_eq!(cfg.app_name, "Coverage Drive");
    assert_eq!(cfg.api_port, 4100);
    assert_eq!(cfg.database_max_connections, 7);
    assert_eq!(cfg.auth_session_ttl_hours, 48);
    assert!(!cfg.auth_pow_enabled);
    assert_eq!(cfg.auth_pow_difficulty, 12);
    assert_eq!(
        cfg.additional_cors_origins,
        vec!["https://a.test".to_string(), "https://b.test".to_string()]
    );
    assert_eq!(
        cfg.webauthn_related_origins,
        vec!["https://rp.test".to_string()]
    );
    assert_eq!(
        cfg.trusted_proxy_cidrs,
        vec!["10.0.0.0/8".to_string(), "192.168.0.0/16".to_string()]
    );
    assert_eq!(cfg.ip_intelligence_provider_specs.len(), 1);
    assert_eq!(cfg.ip_intelligence_timeout_secs, 5);
    assert!(cfg.maxmind_geolite_database_enabled);
    assert!(cfg.maxmind_geolite_eula_accepted);
    assert_eq!(cfg.maxmind_account_id.as_deref(), Some("acct"));
    assert!(cfg.loyalsoldier_geoip_enabled);
    assert!(cfg.product_analytics_enabled);
    assert_eq!(cfg.posthog_host, "https://posthog.test");
    assert!(cfg.billing_fraud_enforcement_enabled);
    assert_eq!(cfg.billing_fraud_step_up_threshold, 55);
    assert_eq!(cfg.otp_provider, "mock");
    assert!(cfg.storage_enabled);
    assert_eq!(cfg.storage_bucket, "coverage-bucket");
    assert_eq!(cfg.storage_access_key.as_deref(), Some("aws-key"));
    assert_eq!(cfg.storage_secret_key.as_deref(), Some("aws-secret"));
    assert!(cfg.scan_enabled);
    assert_eq!(cfg.scan_engine, "clamav");
    assert!(!cfg.scan_fail_open);
    assert_eq!(cfg.request_e2ee_key_id, "k1");
    assert_eq!(
        cfg.security_contact_email.as_deref(),
        Some("sec@example.com")
    );
    assert_eq!(cfg.mtls_port, 4443);
    assert!((cfg.sentry_traces_sample_rate - 0.25).abs() < f32::EPSILON);
}

#[test]
fn secret_manager_enabled_skips_inline_secret_validation() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = EnvGuard::isolated();
    guard.apply(&[
        ("NVBES_ENV", "staging"),
        ("NVBES_SECRET_MANAGER_ENABLED", "true"),
        ("NVBES_WEB_BASE_URL", "https://app.example"),
        ("NVBES_API_BASE_URL", "https://api.example"),
        ("NVBES_BILLING_SUCCESS_URL", "https://app.example/ok"),
        ("NVBES_BILLING_CANCEL_URL", "https://app.example/cancel"),
        (
            "NVBES_BILLING_PORTAL_RETURN_URL",
            "https://app.example/billing",
        ),
        ("NVBES_WEBAUTHN_RP_ID", "example.com"),
        ("NVBES_WEBAUTHN_RP_ORIGIN", "https://example.com"),
    ]);
    let cfg = AppConfig::from_env().expect("secret manager bootstrap");
    assert!(cfg.secret_manager_enabled);
    assert_eq!(cfg.environment, "staging");
}

#[test]
fn strict_mode_requires_public_urls() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENV", "production");
    let err = AppConfig::from_env().expect_err("strict mode must fail");
    assert!(err.contains("NVBES_WEB_BASE_URL") || err.contains("must be set"));
}

#[test]
fn billing_helpers_prefer_explicit_service_base_url() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = EnvGuard::isolated();
    let mut cfg = AppConfig::from_env().expect("defaults");
    cfg.api_base_url = "https://api.default".into();
    cfg.billing_database_url = String::new();
    cfg.database_url = "postgres://primary/db".into();
    assert_eq!(cfg.billing_database_url(), "postgres://primary/db");

    guard.set("NVBES_BILLING_SERVICE_BASE_URL", "https://billing.override");
    assert_eq!(cfg.billing_api_base_url(), "https://billing.override");
}

#[test]
fn case_table_boolean_aliases() {
    let cases = [
        ("true", true),
        ("1", true),
        ("false", false),
        ("0", false),
        ("yes", false),
    ];
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for (raw, expected) in cases {
        let guard = EnvGuard::isolated();
        guard.set("NVBES_DPOP_ENABLED", raw);
        let cfg = AppConfig::from_env().expect("bool parse");
        assert_eq!(cfg.dpop_enabled, expected, "raw={raw}");
        drop(guard);
    }
}

#[test]
fn collects_optional_observability_aliases() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = EnvGuard::isolated();
    guard.apply(&[
        (
            "NVBES_SENTRY_DSN",
            "https://public@example.ingest.sentry.io/1",
        ),
        ("NVBES_PRODUCT_ANALYTICS_ENABLED", "1"),
        ("NVBES_PRODUCT_ANALYTICS_TOKEN", "analytics-token"),
        ("NVBES_ANALYTICS_ID_SALT", "coverage-analytics-salt-32chars"),
    ]);
    let cfg = AppConfig::from_env().expect("aliases");
    assert_eq!(
        cfg.sentry_dsn.as_deref(),
        Some("https://public@example.ingest.sentry.io/1")
    );
    assert!(cfg.product_analytics_enabled);
    assert_eq!(
        cfg.product_analytics_token.as_deref(),
        Some("analytics-token")
    );
}

#[test]
fn mtls_defaults_port_to_api_port_plus_one() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = EnvGuard::isolated();
    guard.set("NVBES_API_PORT", "5000");
    let cfg = AppConfig::from_env().expect("defaults");
    assert_eq!(cfg.mtls_port, 5001);
}
