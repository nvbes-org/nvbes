use super::super::{ConfigError, IdentityConfig, database_url_from_env};
use super::{clear_identity_env, env_lock, set_test_mfa_env};

#[test]
fn database_url_from_env_respects_environment() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
    }
    assert!(matches!(
        database_url_from_env(),
        Err(ConfigError::Missing("NVBES_IDENTITY_DATABASE_URL"))
    ));
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
    }
    assert_eq!(
        database_url_from_env().unwrap(),
        "postgres://localhost/nvbes_identity"
    );
    clear_identity_env();
}

#[test]
fn port_env_overrides_default_bind_addr() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
        std::env::set_var("PORT", "4099");
    }
    let config = IdentityConfig::from_env().unwrap();
    assert_eq!(config.bind_addr.to_string(), "0.0.0.0:4099");
    clear_identity_env();
}

#[test]
fn invalid_mfa_key_version_fails_closed() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "0");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_IDENTITY_MFA_KEY_VERSION"))
    ));
    clear_identity_env();
}

#[test]
fn previous_mfa_version_without_key_is_rejected() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
        std::env::set_var("NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION", "1");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("MFA previous key pair"))
    ));
    clear_identity_env();
}

#[test]
fn production_requires_sentry_after_metrics_token() {
    let _guard = env_lock();
    clear_identity_env();
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
        std::env::set_var(
            "NVBES_IDENTITY_METRICS_TOKEN",
            "production-identity-metrics-token-value",
        );
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Missing("SENTRY_DSN"))
    ));
    clear_identity_env();
}

#[test]
fn production_rejects_authorization_header_with_newlines() {
    let _guard = env_lock();
    clear_identity_env();
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
        std::env::set_var(
            "NVBES_IDENTITY_METRICS_TOKEN",
            "production-identity-metrics-token-value",
        );
        std::env::set_var("SENTRY_DSN", "https://public@o.ingest.sentry.io/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example");
        std::env::set_var("NVBES_OTLP_AUTHORIZATION_HEADER", "Basic abc\n");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_OTLP_AUTHORIZATION_HEADER"))
    ));
    clear_identity_env();
}

#[test]
fn production_rejects_wrong_length_mfa_key_even_when_base64() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var(
            "NVBES_IDENTITY_DATABASE_URL",
            "postgres://identity.test/identity",
        );
        // 16 bytes when decoded — valid base64, wrong key length.
        std::env::set_var(
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "AQEBAQEBAQEBAQEBAQEBAQ==",
        );
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "1");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_IDENTITY_MFA_ENCRYPTION_KEY"))
    ));
    clear_identity_env();
}

#[test]
fn database_pool_zero_and_unparseable_fail_closed() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
        std::env::set_var("NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS", "0");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid(
            "NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS"
        ))
    ));
    unsafe {
        std::env::set_var("NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS", "nope");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid(
            "NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS"
        ))
    ));
    clear_identity_env();
}

#[test]
fn unset_environment_defaults_to_development() {
    let _guard = env_lock();
    clear_identity_env();
    set_test_mfa_env();
    let config = IdentityConfig::from_env().expect("defaults");
    assert_eq!(config.environment, "development");
    clear_identity_env();
}

#[test]
fn blank_optional_env_values_are_ignored() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
        std::env::set_var("NVBES_IDENTITY_TOKEN_ISSUER", "   ");
        std::env::set_var("SENTRY_DSN", "");
    }
    let config = IdentityConfig::from_env().unwrap();
    assert!(config.token_issuer.starts_with("http://"));
    assert!(config.sentry_dsn.is_none());
    clear_identity_env();
}

#[test]
fn test_environment_requires_mfa_encryption_key() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::remove_var("NVBES_IDENTITY_MFA_ENCRYPTION_KEY");
        std::env::remove_var("NVBES_IDENTITY_MFA_KEY_VERSION");
        std::env::remove_var("NVBES_IDENTITY_PUBLIC_SIGNUP");
    }
    let error = IdentityConfig::from_env().expect_err("test must require MFA key");
    assert!(
        error
            .to_string()
            .contains("NVBES_IDENTITY_MFA_ENCRYPTION_KEY")
    );
    unsafe { std::env::remove_var("NVBES_ENVIRONMENT") };
}

#[test]
fn public_signup_defaults_to_environment() {
    let _guard = env_lock();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        set_test_mfa_env();
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
        set_test_mfa_env();
        std::env::set_var(
            "NVBES_IDENTITY_METRICS_TOKEN",
            "production-identity-metrics-token-value",
        );
        std::env::set_var("SENTRY_DSN", "https://key@sentry.example/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example/v1/traces");
        std::env::set_var("NVBES_OTLP_AUTHORIZATION_HEADER", "Basic dXNlcjpwYXNz");
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
