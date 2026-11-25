use super::{AccountConfig, ConfigError};
use std::collections::HashMap;

fn load(development: bool, values: &[(&str, &str)]) -> Result<AccountConfig, ConfigError> {
    let map: HashMap<&str, &str> = values.iter().copied().collect();
    AccountConfig::from_lookup(development, |name| map.get(name).map(|v| (*v).to_owned()))
}

#[test]
fn development_defaults_to_no_browser_origin_and_no_public_origin() {
    let config = load(true, &[]).expect("development config");

    assert_eq!(config.public_origin, None);
    assert!(config.browser_origins == Default::default());
}

#[test]
fn production_requires_browser_origins() {
    assert_eq!(
        load(false, &[]).err(),
        Some(ConfigError::Missing("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON"))
    );
}

#[test]
fn blank_values_count_as_absent() {
    assert_eq!(
        load(false, &[("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON", "  ")]).err(),
        Some(ConfigError::Missing("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON"))
    );
}

#[test]
fn production_requires_public_origin() {
    assert_eq!(
        load(
            false,
            &[(
                "NVBES_ACCOUNT_BROWSER_ORIGINS_JSON",
                r#"["https://account.nvbes.fr"]"#
            )]
        )
        .err(),
        Some(ConfigError::Missing("NVBES_ACCOUNT_PUBLIC_ORIGIN"))
    );
}

#[test]
fn production_rejects_insecure_browser_origins() {
    assert_eq!(
        load(
            false,
            &[
                (
                    "NVBES_ACCOUNT_BROWSER_ORIGINS_JSON",
                    r#"["http://account.nvbes.fr"]"#
                ),
                ("NVBES_ACCOUNT_PUBLIC_ORIGIN", "https://api.nvbes.fr"),
            ]
        )
        .err(),
        Some(ConfigError::Invalid("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON"))
    );
}

#[test]
fn production_config_loads_when_complete() {
    let config = load(
        false,
        &[
            (
                "NVBES_ACCOUNT_BROWSER_ORIGINS_JSON",
                r#"["https://account.nvbes.fr"]"#,
            ),
            ("NVBES_ACCOUNT_PUBLIC_ORIGIN", "https://api.nvbes.fr"),
        ],
    )
    .expect("production config");

    assert_eq!(
        config.public_origin.as_deref(),
        Some("https://api.nvbes.fr")
    );
}

#[test]
fn config_error_messages_name_the_variable() {
    assert_eq!(
        ConfigError::Missing("NVBES_ACCOUNT_PUBLIC_ORIGIN").to_string(),
        "required configuration is missing: NVBES_ACCOUNT_PUBLIC_ORIGIN"
    );
    assert_eq!(
        ConfigError::Invalid("NVBES_ACCOUNT_BROWSER_ORIGINS_JSON").to_string(),
        "configuration is invalid: NVBES_ACCOUNT_BROWSER_ORIGINS_JSON"
    );
}
