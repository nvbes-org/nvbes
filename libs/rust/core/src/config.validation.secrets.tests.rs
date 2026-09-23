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

#[test]
fn missing_key_is_optional_in_development_but_required_strictly() {
    let config = AppConfig {
        auth_factor_encryption_key: None,
        auth_factor_encryption_key_version: 1,
        ..AppConfig::default()
    };
    validate_auth_factor_encryption(&config, false).expect("dev ok");
    let err = validate_auth_factor_encryption(&config, true).expect_err("strict");
    assert!(err.contains("required outside development"));
}

#[test]
fn accepts_valid_factor_key_and_rejects_short_decoded_material() {
    let short = AppConfig {
        auth_factor_encryption_key: Some(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            [0_u8; 16],
        )),
        auth_factor_encryption_key_version: 1,
        ..AppConfig::default()
    };
    let err = validate_auth_factor_encryption(&short, false).expect_err("short");
    assert!(err.contains("32 bytes"));

    let ok = AppConfig {
        auth_factor_encryption_key: Some(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            [7_u8; 32],
        )),
        auth_factor_encryption_key_version: 2,
        ..AppConfig::default()
    };
    validate_auth_factor_encryption(&ok, true).expect("valid key");
}
