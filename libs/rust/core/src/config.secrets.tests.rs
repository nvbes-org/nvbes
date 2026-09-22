use base64::Engine;
use serde_json::{Map, Value, json};

use super::{AppConfig, assign_optional, assign_required, decoded_secret_object, required_env};

#[test]
fn resolve_is_noop_when_secret_manager_disabled() {
    let mut config = AppConfig {
        secret_manager_enabled: false,
        ..AppConfig::default()
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime
        .block_on(config.resolve_from_secret_manager())
        .expect("disabled secret manager must succeed");
}

#[test]
fn decoded_secret_object_accepts_plain_json_object() {
    let payload = json!({
        "database_url": "postgres://db",
        "jwt_secret": "secret-value-with-enough-length-1234"
    });
    let object = decoded_secret_object(payload).expect("object");
    assert_eq!(
        object.get("database_url").and_then(Value::as_str),
        Some("postgres://db")
    );
}

#[test]
fn decoded_secret_object_decodes_base64_data_field() {
    let inner = json!({
        "database_url": "postgres://decoded",
        "billing_database_url": "postgres://billing",
        "jwt_secret": "decoded-jwt-secret-with-32chars-ok"
    });
    let encoded = base64::engine::general_purpose::STANDARD.encode(inner.to_string());
    let payload = json!({ "data": encoded });
    let object = decoded_secret_object(payload).expect("decoded");
    assert_eq!(
        object.get("database_url").and_then(Value::as_str),
        Some("postgres://decoded")
    );
}

#[test]
fn decoded_secret_object_rejects_invalid_payloads() {
    assert!(decoded_secret_object(json!("not-an-object")).is_err());
    assert!(decoded_secret_object(json!({ "data": "!!!not-base64!!!" })).is_err());
    let encoded_array = base64::engine::general_purpose::STANDARD.encode("[1,2,3]");
    assert!(decoded_secret_object(json!({ "data": encoded_array })).is_err());
}

#[test]
fn apply_secret_values_assigns_required_and_optional_fields() {
    let mut config = AppConfig::default();
    let mut secrets = Map::new();
    secrets.insert("database_url".into(), json!("postgres://primary"));
    secrets.insert("billing_database_url".into(), json!("postgres://billing"));
    secrets.insert(
        "jwt_secret".into(),
        json!("secret-manager-jwt-with-32-chars!!"),
    );
    secrets.insert("stripe_secret_key".into(), json!("sk_test_from_sm"));
    secrets.insert("sentry_dsn".into(), Value::Null);
    secrets.insert("twilio_account_sid".into(), json!("ACxxxxxxxx"));

    config.apply_secret_values(&secrets).expect("apply secrets");
    assert_eq!(config.database_url, "postgres://primary");
    assert_eq!(config.billing_database_url, "postgres://billing");
    assert_eq!(config.jwt_secret, "secret-manager-jwt-with-32-chars!!");
    assert_eq!(config.stripe_secret_key.as_deref(), Some("sk_test_from_sm"));
    assert_eq!(config.sentry_dsn, None);
    assert_eq!(config.twilio_account_sid.as_deref(), Some("ACxxxxxxxx"));
}

#[test]
fn assign_helpers_reject_empty_and_non_string_values() {
    let mut secrets = Map::new();
    secrets.insert("database_url".into(), json!(""));
    let mut target = String::from("keep");
    let err = assign_required(&secrets, "database_url", &mut target).unwrap_err();
    assert!(err.contains("non-empty"));
    assert_eq!(target, "keep");

    secrets.insert("stripe_secret_key".into(), json!(42));
    let mut optional = Some("old".to_string());
    let err = assign_optional(&secrets, "stripe_secret_key", &mut optional).unwrap_err();
    assert!(err.contains("non-empty string or null"));
}

#[test]
fn assign_required_ignores_missing_keys() {
    let secrets = Map::new();
    let mut target = String::from("unchanged");
    assign_required(&secrets, "database_url", &mut target).expect("missing ok");
    assert_eq!(target, "unchanged");
}

#[test]
fn required_env_reports_missing_variable() {
    let name = "NVBES_CORE_SECRETS_TEST_MISSING_VAR_XYZ";
    unsafe { std::env::remove_var(name) };
    let err = required_env(name).unwrap_err();
    assert!(err.contains(name));
}

#[tokio::test]
async fn resolve_rejects_invalid_secret_manager_region_without_network() {
    let name_region = "NVBES_SECRET_MANAGER_REGION";
    let name_secret = "NVBES_SECRET_MANAGER_SECRET_ID";
    let saved_region = std::env::var_os(name_region);
    let saved_secret = std::env::var_os(name_secret);
    unsafe {
        std::env::set_var(name_secret, "test-secret-id");
        std::env::set_var(name_region, "invalid-region");
    }

    let mut config = AppConfig {
        secret_manager_enabled: true,
        ..AppConfig::default()
    };
    let err = config
        .resolve_from_secret_manager()
        .await
        .expect_err("invalid region");
    assert!(err.contains("NVBES_SECRET_MANAGER_REGION"));

    unsafe {
        match saved_region {
            Some(value) => std::env::set_var(name_region, value),
            None => std::env::remove_var(name_region),
        }
        match saved_secret {
            Some(value) => std::env::set_var(name_secret, value),
            None => std::env::remove_var(name_secret),
        }
    }
}
