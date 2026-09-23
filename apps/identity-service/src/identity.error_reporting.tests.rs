use super::{APP_NAME, ErrorReportingRuntimeConfig};
use crate::database::database_test_support::test_env_lock;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    test_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn clear() {
    for name in [
        "NVBES_ENVIRONMENT",
        "SENTRY_DSN",
        "SENTRY_TRACES_SAMPLE_RATE",
    ] {
        // SAFETY: serialized by `test_env_lock` for test-only env mutation.
        unsafe { std::env::remove_var(name) };
    }
}

#[test]
fn app_name_is_stable() {
    assert_eq!(APP_NAME, "nvbes-identity-service");
}

#[test]
fn from_env_requires_environment_and_https_dsn() {
    let _guard = env_lock();
    clear();
    assert!(ErrorReportingRuntimeConfig::from_env().is_err());

    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var("SENTRY_DSN", "http://public@o.ingest.sentry.io/1");
    }
    assert!(
        ErrorReportingRuntimeConfig::from_env()
            .unwrap_err()
            .to_string()
            .contains("HTTPS")
    );

    unsafe {
        std::env::set_var("SENTRY_DSN", "https://public@o.ingest.sentry.io/1");
        std::env::set_var("SENTRY_TRACES_SAMPLE_RATE", "1.5");
    }
    assert!(
        ErrorReportingRuntimeConfig::from_env()
            .unwrap_err()
            .to_string()
            .contains("SENTRY_TRACES_SAMPLE_RATE")
    );

    unsafe {
        std::env::set_var("SENTRY_TRACES_SAMPLE_RATE", "0.2");
    }
    let config = ErrorReportingRuntimeConfig::from_env().unwrap();
    assert_eq!(config.environment, "production");
    assert!(config.dsn.starts_with("https://"));
    assert!((config.traces_sample_rate - 0.2).abs() < f32::EPSILON);
    clear();
}

#[test]
fn smoke_returns_structured_result_in_development_without_network() {
    let _guard = env_lock();
    clear();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "development");
        std::env::set_var("SENTRY_DSN", "https://public@o.ingest.sentry.io/1");
        std::env::set_var("SENTRY_TRACES_SAMPLE_RATE", "0");
    }
    let config = ErrorReportingRuntimeConfig::from_env().expect("config");
    let _guard = super::init(&config);
    let result = super::smoke(&config);
    assert!(result.configured);
    clear();
}
