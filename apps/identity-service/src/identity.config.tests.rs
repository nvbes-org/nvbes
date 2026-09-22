use std::sync::{Mutex, OnceLock};

use super::IdentityConfig;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn production_requires_an_explicit_database() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::remove_var("NVBES_IDENTITY_DATABASE_URL");
        std::env::remove_var("NVBES_IDENTITY_MFA_ENCRYPTION_KEY");
        std::env::remove_var("NVBES_IDENTITY_MFA_KEY_VERSION");
    }
    let error = IdentityConfig::from_env().expect_err("production must fail closed");
    assert!(error.to_string().contains("NVBES_IDENTITY_DATABASE_URL"));
    unsafe { std::env::remove_var("NVBES_ENVIRONMENT") };
}

#[test]
fn production_requires_an_exact_32_byte_mfa_key() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var("NVBES_IDENTITY_DATABASE_URL", "postgres://identity.test/db");
        std::env::set_var("NVBES_IDENTITY_MFA_ENCRYPTION_KEY", "too-short");
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "1");
    }
    let error = IdentityConfig::from_env().expect_err("invalid encryption key must fail closed");
    assert!(
        error
            .to_string()
            .contains("NVBES_IDENTITY_MFA_ENCRYPTION_KEY")
    );
    unsafe {
        std::env::remove_var("NVBES_ENVIRONMENT");
        std::env::remove_var("NVBES_IDENTITY_DATABASE_URL");
        std::env::remove_var("NVBES_IDENTITY_MFA_ENCRYPTION_KEY");
        std::env::remove_var("NVBES_IDENTITY_MFA_KEY_VERSION");
    }
}

#[test]
fn previous_mfa_key_requires_a_distinct_complete_pair() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var(
            "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY",
            "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=",
        );
        std::env::remove_var("NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION");
    }
    assert!(IdentityConfig::from_env().is_err());
    unsafe {
        std::env::remove_var("NVBES_ENVIRONMENT");
        std::env::remove_var("NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY");
    }
}

#[test]
fn database_pool_is_bounded() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS", "21");
    }
    let error = IdentityConfig::from_env().expect_err("oversized pool must fail");
    assert!(
        error
            .to_string()
            .contains("NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS")
    );
    unsafe {
        std::env::remove_var("NVBES_ENVIRONMENT");
        std::env::remove_var("NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS");
    }
}

#[test]
fn production_requires_authenticated_observability() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var(
            "NVBES_IDENTITY_DATABASE_URL",
            "postgres://identity.test/identity",
        );
        std::env::set_var(
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=",
        );
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "1");
        std::env::remove_var("NVBES_IDENTITY_METRICS_TOKEN");
    }
    let error = IdentityConfig::from_env().expect_err("production metrics must fail closed");
    assert!(error.to_string().contains("NVBES_IDENTITY_METRICS_TOKEN"));
    unsafe {
        for name in [
            "NVBES_ENVIRONMENT",
            "NVBES_IDENTITY_DATABASE_URL",
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "NVBES_IDENTITY_MFA_KEY_VERSION",
        ] {
            std::env::remove_var(name);
        }
    }
}

#[test]
fn public_signup_defaults_to_environment() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::remove_var("NVBES_IDENTITY_PUBLIC_SIGNUP");
        std::env::remove_var("NVBES_IDENTITY_LOGIN_URL");
        std::env::remove_var("NVBES_IDENTITY_SESSION_COOKIE_SECURE");
    }
    let config = IdentityConfig::from_env().expect("test config");
    assert!(config.public_signup_enabled);
    assert!(!config.session_cookie_secure);
    assert!(config.login_url.is_empty());
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var("NVBES_IDENTITY_DATABASE_URL", "postgres://identity.test/db");
        std::env::set_var(
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=",
        );
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "1");
        std::env::set_var(
            "NVBES_IDENTITY_METRICS_TOKEN",
            "production-identity-metrics-token-value",
        );
        std::env::set_var("SENTRY_DSN", "https://key@sentry.example/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example/v1/traces");
        std::env::set_var(
            "NVBES_OTLP_AUTHORIZATION_HEADER",
            "Basic dXNlcjpwYXNz",
        );
        std::env::remove_var("NVBES_IDENTITY_PUBLIC_SIGNUP");
    }
    let production = IdentityConfig::from_env().expect("production config");
    assert!(!production.public_signup_enabled);
    assert!(production.session_cookie_secure);
    unsafe {
        for name in [
            "NVBES_ENVIRONMENT",
            "NVBES_IDENTITY_DATABASE_URL",
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "NVBES_IDENTITY_MFA_KEY_VERSION",
            "NVBES_IDENTITY_METRICS_TOKEN",
            "SENTRY_DSN",
            "NVBES_OTLP_ENDPOINT",
            "NVBES_OTLP_AUTHORIZATION_HEADER",
        ] {
            std::env::remove_var(name);
        }
    }
}
