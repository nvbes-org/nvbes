use super::validate_auth_factor_encryption;
use crate::config::AppConfig;

#[test]
fn rejects_zero_key_version() {
    let config = AppConfig {
        auth_factor_encryption_key_version: 0,
        ..AppConfig::default()
    };
    let err = validate_auth_factor_encryption(&config, false).expect_err("version");
    assert!(err.contains("KEY_VERSION"));
}

#[test]
fn rejects_invalid_base64_factor_key() {
    let config = AppConfig {
        auth_factor_encryption_key: Some("not-valid-base64!!!".to_string()),
        auth_factor_encryption_key_version: 1,
        ..AppConfig::default()
    };
    let err = validate_auth_factor_encryption(&config, false).expect_err("base64");
    assert!(err.contains("base64"));
}
