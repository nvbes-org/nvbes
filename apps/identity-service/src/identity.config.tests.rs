use super::{ConfigError, IdentityConfig, database_url_from_env};
use crate::database::database_test_support::test_env_lock;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    test_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn clear_identity_env() {
    for name in [
        "NVBES_ENVIRONMENT",
        "NVBES_IDENTITY_DATABASE_URL",
        "NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS",
        "NVBES_IDENTITY_BIND_ADDR",
        "PORT",
        "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
        "NVBES_IDENTITY_MFA_KEY_VERSION",
        "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY",
        "NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION",
        "NVBES_IDENTITY_METRICS_TOKEN",
        "NVBES_IDENTITY_TOKEN_ISSUER",
        "SENTRY_DSN",
        "SENTRY_TRACES_SAMPLE_RATE",
        "NVBES_OTLP_ENDPOINT",
        "NVBES_OTLP_AUTHORIZATION_HEADER",
    ] {
        // SAFETY: serialized by `test_env_lock` for test-only env mutation.
        unsafe { std::env::remove_var(name) };
    }
}

#[test]
fn production_requires_an_explicit_database() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
    }
    let error = IdentityConfig::from_env().expect_err("production must fail closed");
    assert!(error.to_string().contains("NVBES_IDENTITY_DATABASE_URL"));
    clear_identity_env();
}

#[test]
fn production_requires_an_exact_32_byte_mfa_key() {
    let _guard = env_lock();
    clear_identity_env();
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
    clear_identity_env();
}

#[test]
fn previous_mfa_key_requires_a_distinct_complete_pair() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var(
            "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY",
            "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=",
        );
    }
    assert!(IdentityConfig::from_env().is_err());
    clear_identity_env();
}

#[test]
fn database_pool_is_bounded() {
    let _guard = env_lock();
    clear_identity_env();
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
    clear_identity_env();
}

#[test]
fn production_requires_authenticated_observability() {
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
    }
    let error = IdentityConfig::from_env().expect_err("production metrics must fail closed");
    assert!(error.to_string().contains("NVBES_IDENTITY_METRICS_TOKEN"));
    clear_identity_env();
}

#[test]
fn development_config_loads_with_defaults() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "development");
    }
    let config = IdentityConfig::from_env().expect("development defaults");
    assert_eq!(config.environment, "development");
    assert_eq!(config.database_max_connections, 5);
    assert_eq!(config.mfa_key_version, 1);
    assert!(config.metrics_token.len() >= 32);
    assert!(config.mfa_previous_encryption_key.is_none());
    clear_identity_env();
}

#[test]
fn invalid_bind_addr_and_sentry_sample_rate_fail_closed() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_IDENTITY_BIND_ADDR", "not-an-addr");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_IDENTITY_BIND_ADDR"))
    ));

    unsafe {
        std::env::set_var("NVBES_IDENTITY_BIND_ADDR", "127.0.0.1:3060");
        std::env::set_var("SENTRY_TRACES_SAMPLE_RATE", "2.5");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("SENTRY_TRACES_SAMPLE_RATE"))
    ));
    clear_identity_env();
}

#[test]
fn metrics_token_rejects_short_and_newline_values() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_IDENTITY_METRICS_TOKEN", "too-short");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_IDENTITY_METRICS_TOKEN"))
    ));

    unsafe {
        std::env::set_var(
            "NVBES_IDENTITY_METRICS_TOKEN",
            "development-identity-metrics-token-value\n",
        );
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_IDENTITY_METRICS_TOKEN"))
    ));
    clear_identity_env();
}

#[test]
fn previous_mfa_key_versions_must_be_distinct() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "2");
        std::env::set_var(
            "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY",
            "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=",
        );
        std::env::set_var("NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION", "2");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("MFA key versions must be distinct"))
    ));
    clear_identity_env();
}

#[test]
fn previous_mfa_key_pair_loads_when_complete_and_distinct() {
    let _guard = env_lock();
    clear_identity_env();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_IDENTITY_MFA_KEY_VERSION", "2");
        std::env::set_var(
            "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY",
            "AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=",
        );
        std::env::set_var("NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION", "1");
    }
    let config = IdentityConfig::from_env().expect("previous key pair");
    assert_eq!(config.mfa_key_version, 2);
    assert_eq!(config.mfa_previous_key_version, Some(1));
    assert!(config.mfa_previous_encryption_key.is_some());
    clear_identity_env();
}

#[test]
fn production_rejects_invalid_observability_endpoints() {
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
        std::env::set_var("SENTRY_DSN", "http://public@o.ingest.sentry.io/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example");
        std::env::set_var("NVBES_OTLP_AUTHORIZATION_HEADER", "Basic abc");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("SENTRY_DSN"))
    ));

    unsafe {
        std::env::set_var("SENTRY_DSN", "https://public@o.ingest.sentry.io/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "http://otlp.example");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_OTLP_ENDPOINT"))
    ));

    unsafe {
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example");
        std::env::set_var("NVBES_OTLP_AUTHORIZATION_HEADER", "Bearer token");
    }
    assert!(matches!(
        IdentityConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_OTLP_AUTHORIZATION_HEADER"))
    ));
    clear_identity_env();
}

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
        std::env::set_var("NVBES_IDENTITY_TOKEN_ISSUER", "   ");
        std::env::set_var("SENTRY_DSN", "");
    }
    let config = IdentityConfig::from_env().unwrap();
    assert!(config.token_issuer.starts_with("http://"));
    assert!(config.sentry_dsn.is_none());
    clear_identity_env();
}
