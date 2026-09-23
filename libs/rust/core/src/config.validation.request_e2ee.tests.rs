use super::validate_request_e2ee;
use crate::config::AppConfig;

#[test]
fn enabled_e2ee_requires_secret_and_key_id() {
    let missing_secret = AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_key_id: "kid".to_string(),
        ..AppConfig::default()
    };
    let err = validate_request_e2ee(&missing_secret, false).expect_err("secret");
    assert!(err.contains("NVBES_REQUEST_E2EE_SECRET"));

    let empty_key = AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_secret: Some("0123456789abcdef0123456789abcdef".to_string()),
        request_e2ee_key_id: "   ".to_string(),
        ..AppConfig::default()
    };
    let err = validate_request_e2ee(&empty_key, false).expect_err("key id");
    assert!(err.contains("NVBES_REQUEST_E2EE_KEY_ID"));
}

#[test]
fn strict_mode_rejects_default_key_id() {
    let config = AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_secret: Some("0123456789abcdef0123456789abcdef".to_string()),
        request_e2ee_key_id: "default".to_string(),
        ..AppConfig::default()
    };
    let err = validate_request_e2ee(&config, true).expect_err("default key");
    assert!(err.contains("default"));
}

#[test]
fn disabled_e2ee_is_a_noop() {
    validate_request_e2ee(&AppConfig::default(), true).expect("disabled ok");
}
