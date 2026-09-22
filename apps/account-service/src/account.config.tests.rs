use std::sync::{Mutex, OnceLock};

use super::{AccountConfig, ConfigError, required_database_url};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn clear() {
    for name in [
        "NVBES_ENVIRONMENT",
        "NVBES_ACCOUNT_DATABASE_URL",
        "NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS",
        "NVBES_ACCOUNT_BIND_ADDR",
        "PORT",
        "NVBES_IDENTITY_TOKEN_ISSUER",
        "NVBES_ACCOUNT_TOKEN_AUDIENCE",
        "NVBES_IDENTITY_TOKEN_KEY_ID",
        "NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM",
        "NVBES_ACCOUNT_METRICS_TOKEN",
        "SENTRY_DSN",
        "SENTRY_TRACES_SAMPLE_RATE",
        "NVBES_OTLP_ENDPOINT",
        "NVBES_OTLP_AUTHORIZATION_HEADER",
    ] {
        unsafe { std::env::remove_var(name) };
    }
}

fn set_development_token_material() {
    unsafe {
        std::env::set_var("NVBES_IDENTITY_TOKEN_KEY_ID", "identity-key-1");
        std::env::set_var(
            "NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM",
            "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwEKbtGra6bWscKp5s8i3\njvE0mUrZV54vaDWEfYxnjJYc6TNPBUuHlNUOv44eePZ+TxLQ9wPYSwuhSJIPAX7c\nAkDDdzVJy36lUVTjKGND7PZtgv6ItPQb6yM7YNyM7+QHZXL0fvB5q7O1AasgT+sF\ns0ffePjSyi9QpOww8TqcgePyXN3anUmB8pwoaJQfJOOLE1sJ3zsfw+n3nG+31Lpg\nDQBwYQWrKRRH/R7aYRFoZu1ZRFINfYXVmOGUuehcYkAprNs1dte6szKyyc2zhUJi\nTUez2NsFzgzx1Q1ssxYOMbQcVqNjux5l2aC9i5VcPi7gRHWGKiN77sSjnz90vqSu\nYwIDAQAB\n-----END PUBLIC KEY-----\n",
        );
    }
}

#[test]
fn development_config_loads_with_defaults() {
    let _guard = env_lock();
    clear();
    set_development_token_material();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "development");
    }
    let config = AccountConfig::from_env().expect("development defaults");
    assert_eq!(config.environment, "development");
    assert_eq!(config.database_max_connections, 5);
    assert_eq!(config.token_audience, "nvbes-account");
    assert!(config.metrics_token.len() >= 32);
    clear();
}

#[test]
fn production_requires_database_url() {
    let _guard = env_lock();
    clear();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Missing("NVBES_ACCOUNT_DATABASE_URL"))
    ));
    clear();
}

#[test]
fn invalid_bind_addr_and_pool_size_fail_closed() {
    let _guard = env_lock();
    clear();
    set_development_token_material();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_ACCOUNT_BIND_ADDR", "bad");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_ACCOUNT_BIND_ADDR"))
    ));

    unsafe {
        std::env::set_var("NVBES_ACCOUNT_BIND_ADDR", "127.0.0.1:3070");
        std::env::set_var("NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS", "0");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid(
            "NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS"
        ))
    ));
    clear();
}

#[test]
fn token_contract_rejects_insecure_issuer_and_bad_identifiers() {
    let _guard = env_lock();
    clear();
    set_development_token_material();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var(
            "NVBES_ACCOUNT_DATABASE_URL",
            "postgres://account.test/account",
        );
        std::env::set_var("NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS", "5");
        std::env::set_var("NVBES_IDENTITY_TOKEN_ISSUER", "http://identity.test");
        std::env::set_var("NVBES_ACCOUNT_TOKEN_AUDIENCE", "nvbes-account");
        std::env::set_var(
            "NVBES_ACCOUNT_METRICS_TOKEN",
            "production-account-metrics-token-value",
        );
        std::env::set_var("SENTRY_DSN", "https://public@o.ingest.sentry.io/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example");
        std::env::set_var("NVBES_OTLP_AUTHORIZATION_HEADER", "Basic abc");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_IDENTITY_TOKEN_ISSUER"))
    ));

    unsafe {
        std::env::set_var("NVBES_IDENTITY_TOKEN_ISSUER", "https://identity.test");
        std::env::set_var("NVBES_ACCOUNT_TOKEN_AUDIENCE", "ab");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_ACCOUNT_TOKEN_AUDIENCE"))
    ));
    clear();
}

#[test]
fn observability_and_metrics_token_validation_fail_closed() {
    let _guard = env_lock();
    clear();
    set_development_token_material();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("NVBES_ACCOUNT_METRICS_TOKEN", "short");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_ACCOUNT_METRICS_TOKEN"))
    ));

    unsafe {
        std::env::set_var(
            "NVBES_ACCOUNT_METRICS_TOKEN",
            "development-account-metrics-token-value",
        );
        std::env::set_var("SENTRY_TRACES_SAMPLE_RATE", "nope");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("SENTRY_TRACES_SAMPLE_RATE"))
    ));
    clear();
}

#[test]
fn production_requires_https_observability() {
    let _guard = env_lock();
    clear();
    set_development_token_material();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "production");
        std::env::set_var(
            "NVBES_ACCOUNT_DATABASE_URL",
            "postgres://account.test/account",
        );
        std::env::set_var("NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS", "5");
        std::env::set_var("NVBES_IDENTITY_TOKEN_ISSUER", "https://identity.test");
        std::env::set_var("NVBES_ACCOUNT_TOKEN_AUDIENCE", "nvbes-account");
        std::env::set_var(
            "NVBES_ACCOUNT_METRICS_TOKEN",
            "production-account-metrics-token-value",
        );
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("SENTRY_DSN"))
    ));

    unsafe {
        std::env::set_var("SENTRY_DSN", "https://public@o.ingest.sentry.io/1");
        std::env::set_var("NVBES_OTLP_ENDPOINT", "http://otlp.example");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_OTLP_ENDPOINT"))
    ));

    unsafe {
        std::env::set_var("NVBES_OTLP_ENDPOINT", "https://otlp.example");
        std::env::set_var("NVBES_OTLP_AUTHORIZATION_HEADER", "Token abc");
    }
    assert!(matches!(
        AccountConfig::from_env(),
        Err(ConfigError::Invalid("NVBES_OTLP_AUTHORIZATION_HEADER"))
    ));
    clear();
}

#[test]
fn required_database_url_and_port_defaults() {
    let _guard = env_lock();
    clear();
    assert!(matches!(
        required_database_url(),
        Err(ConfigError::Missing("NVBES_ACCOUNT_DATABASE_URL"))
    ));
    set_development_token_material();
    unsafe {
        std::env::set_var("NVBES_ENVIRONMENT", "test");
        std::env::set_var("PORT", "4010");
    }
    let config = AccountConfig::from_env().unwrap();
    assert_eq!(config.bind_addr.to_string(), "0.0.0.0:4010");
    assert!(config.token_issuer.ends_with("identity.local") || !config.token_issuer.is_empty());
    clear();
}
