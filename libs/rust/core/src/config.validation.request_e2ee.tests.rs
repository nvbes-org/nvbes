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

#[test]
fn enabled_e2ee_accepts_strong_secret_and_custom_key_id() {
    let config = AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_secret: Some("0123456789abcdef0123456789abcdef".to_string()),
        request_e2ee_key_id: "v1".to_string(),
        ..AppConfig::default()
    };
    validate_request_e2ee(&config, true).expect("valid e2ee");
    validate_request_e2ee(
        &AppConfig {
            request_e2ee_key_id: "default".to_string(),
            ..config.clone()
        },
        false,
    )
    .expect("default key id allowed outside strict mode");
}

#[test]
fn required_without_enabled_is_rejected() {
    let config = AppConfig {
        request_e2ee_required: true,
        request_e2ee_enabled: false,
        ..AppConfig::default()
    };
    let err = validate_request_e2ee(&config, false).expect_err("required");
    assert!(err.contains("NVBES_REQUEST_E2EE_ENABLED"));
}

#[test]
fn short_secret_is_rejected() {
    let config = AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_secret: Some("too-short".to_string()),
        request_e2ee_key_id: "kid".to_string(),
        ..AppConfig::default()
    };
    let err = validate_request_e2ee(&config, false).expect_err("short");
    assert!(err.contains("32 characters"));
}
