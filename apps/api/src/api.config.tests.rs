use super::*;
use std::collections::HashMap;

const ORIGINS: &str = r#"["https://account.nvbes.fr"]"#;

fn production(overrides: &[(&str, Option<&str>)]) -> Result<ApiConfig, ConfigError> {
    let mut values: HashMap<&str, &str> = HashMap::from([
        ("NVBES_ENVIRONMENT", "production"),
        ("DATABASE_URL", "postgres://db.internal/nvbes"),
        ("NVBES_TRUSTED_PROXY_CIDRS", "100.64.0.0/10, 10.0.0.0/8"),
        ("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON", ORIGINS),
        ("NVBES_ACCOUNT_PUBLIC_ORIGIN", "https://api.nvbes.fr"),
    ]);
    for (name, value) in overrides {
        match value {
            Some(value) => values.insert(name, value),
            None => values.remove(name),
        };
    }
    ApiConfig::from_lookup(&|name| values.get(name).map(|value| (*value).to_owned()))
}

#[test]
fn complete_production_configuration_loads() {
    let config = production(&[]).expect("production config");
    assert!(!config.development());
    assert_eq!(config.bind_addr.to_string(), "0.0.0.0:8080");
    assert_eq!(
        config.trusted_proxy_cidrs,
        vec!["100.64.0.0/10".to_owned(), "10.0.0.0/8".to_owned()]
    );
}

#[test]
fn production_never_invents_a_database_url() {
    assert_eq!(
        production(&[("DATABASE_URL", None)]).err(),
        Some(ConfigError::Missing("DATABASE_URL"))
    );
}

#[test]
fn production_requires_explicit_trusted_proxies() {
    assert_eq!(
        production(&[("NVBES_TRUSTED_PROXY_CIDRS", None)]).err(),
        Some(ConfigError::Missing("NVBES_TRUSTED_PROXY_CIDRS"))
    );
    assert_eq!(
        production(&[("NVBES_TRUSTED_PROXY_CIDRS", Some("10.0.0.0/8, nope"))]).err(),
        Some(ConfigError::Invalid("NVBES_TRUSTED_PROXY_CIDRS"))
    );
}

#[test]
fn production_requires_account_origins() {
    assert_eq!(
        production(&[("NVBES_ACCOUNT_PUBLIC_ORIGIN", None)]).err(),
        Some(ConfigError::Missing("NVBES_ACCOUNT_PUBLIC_ORIGIN"))
    );
}

#[test]
fn development_falls_back_to_local_defaults() {
    let config = ApiConfig::from_lookup(&|_| None).expect("development config");
    assert!(config.development());
    assert_eq!(config.bind_addr.to_string(), "127.0.0.1:8080");
    assert!(config.trusted_proxy_cidrs.is_empty());
}

#[test]
fn blank_values_count_as_absent() {
    assert_eq!(
        production(&[("DATABASE_URL", Some("   "))]).err(),
        Some(ConfigError::Missing("DATABASE_URL"))
    );
}
