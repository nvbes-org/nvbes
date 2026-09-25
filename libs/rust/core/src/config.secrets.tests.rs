use base64::Engine;
use serde_json::{Map, Value, json};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::{AppConfig, assign_optional, assign_required, decoded_secret_object, required_env};

fn secret_manager_env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

#[test]
fn apply_secret_values_covers_optional_secret_fields() {
    let mut config = AppConfig::default();
    let mut secrets = Map::new();
    secrets.insert("database_url".into(), json!("postgres://primary"));
    secrets.insert("billing_database_url".into(), json!("postgres://billing"));
    secrets.insert(
        "jwt_secret".into(),
        json!("secret-manager-jwt-with-32-chars!!"),
    );
    secrets.insert("auth_password_pepper".into(), json!("pepper-value"));
    secrets.insert(
        "auth_factor_encryption_key".into(),
        json!("ZmFrZS1mYWN0b3ItZW5jcnlwdGlvbi1rZXk="),
    );
    secrets.insert("stripe_webhook_secret".into(), json!("whsec_test"));
    secrets.insert("mollie_api_key".into(), json!("test_mollie"));
    secrets.insert("otlp_authorization_header".into(), json!("Bearer otlp"));
    secrets.insert("product_analytics_token".into(), json!("phc_token"));
    secrets.insert("analytics_id_salt".into(), json!("analytics-salt-value"));
    secrets.insert(
        "profiling_basic_auth_password".into(),
        json!("profile-pass"),
    );
    secrets.insert(
        "observability_internal_token".into(),
        json!("observability-internal-token-value-32"),
    );
    secrets.insert("twilio_auth_token".into(), json!("twilio-token"));
    secrets.insert("storage_access_key".into(), json!("access"));
    secrets.insert("storage_secret_key".into(), json!("secret"));
    secrets.insert("maxmind_account_id".into(), json!("12345"));
    secrets.insert("maxmind_license_key".into(), json!("license"));
    secrets.insert(
        "request_e2ee_secret".into(),
        json!("0123456789abcdef0123456789abcdef"),
    );

    config.apply_secret_values(&secrets).expect("apply");
    assert_eq!(config.auth_password_pepper.as_deref(), Some("pepper-value"));
    assert_eq!(config.mollie_api_key.as_deref(), Some("test_mollie"));
    assert_eq!(
        config.request_e2ee_secret.as_deref(),
        Some("0123456789abcdef0123456789abcdef")
    );
}

#[tokio::test]
async fn resolve_requires_auth_token_when_secret_manager_enabled() {
    // Must use SecretManagerEnv (global lock) — raw set_var races llvm-cov/nextest workers.
    let _env = SecretManagerEnv::install(&[
        ("NVBES_SECRET_MANAGER_SECRET_ID", Some("test-secret-id")),
        ("NVBES_SECRET_MANAGER_REGION", Some("fr-par")),
        ("NVBES_SECRET_MANAGER_AUTH_TOKEN", None),
        ("SCW_SECRET_KEY", None),
    ]);

    let mut config = AppConfig {
        secret_manager_enabled: true,
        ..AppConfig::default()
    };
    let err = config
        .resolve_from_secret_manager()
        .await
        .expect_err("missing auth token");
    assert!(err.contains("NVBES_SECRET_MANAGER_AUTH_TOKEN"));
}

fn development_config_ready_for_secret_apply() -> AppConfig {
    AppConfig {
        environment: "development".into(),
        web_base_url: "http://localhost:5173".into(),
        api_base_url: "http://localhost:3000".into(),
        billing_default_success_url: "http://localhost:5173/billing/success".into(),
        billing_default_cancel_url: "http://localhost:5173/billing/cancel".into(),
        billing_default_portal_return_url: "http://localhost:5173/billing".into(),
        webauthn_rp_origin: "http://localhost:5173".into(),
        webauthn_rp_id: "localhost".into(),
        stripe_api_base_url: "https://api.stripe.com".into(),
        mollie_api_base_url: "https://api.mollie.com".into(),
        twilio_api_base_url: "https://verify.twilio.com".into(),
        otp_provider: "mock".into(),
        auth_session_idle_ttl_minutes: 30,
        auth_verification_resend_cooldown_seconds: 60,
        auth_unverified_account_ttl_days: 7,
        profiling_sample_rate_hz: 100,
        auth_factor_encryption_key_version: 1,
        ip_intelligence_timeout_secs: 5,
        ip_intelligence_cache_ttl_hours: 24,
        secret_manager_enabled: true,
        ..AppConfig::default()
    }
}

struct SecretManagerEnv {
    _guard: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl SecretManagerEnv {
    fn install(pairs: &[(&str, Option<&str>)]) -> Self {
        let guard = secret_manager_env_lock();
        let names = [
            "NVBES_SECRET_MANAGER_SECRET_ID",
            "NVBES_SECRET_MANAGER_REGION",
            "NVBES_SECRET_MANAGER_AUTH_TOKEN",
            "NVBES_SECRET_MANAGER_API_BASE_URL",
            "SCW_SECRET_KEY",
        ];
        let saved = names
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        for name in names {
            // SAFETY: serialized by `secret_manager_env_lock` for test-only env mutation.
            unsafe { std::env::remove_var(name) };
        }
        for (name, value) in pairs {
            match value {
                // SAFETY: serialized by `secret_manager_env_lock` for test-only env mutation.
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
        Self {
            _guard: guard,
            saved,
        }
    }
}

impl Drop for SecretManagerEnv {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            match value {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

#[tokio::test]
async fn resolve_requires_secret_id_when_secret_manager_enabled() {
    let _env = SecretManagerEnv::install(&[
        ("NVBES_SECRET_MANAGER_SECRET_ID", None),
        ("NVBES_SECRET_MANAGER_REGION", Some("fr-par")),
        ("NVBES_SECRET_MANAGER_AUTH_TOKEN", Some("token")),
    ]);
    let mut config = development_config_ready_for_secret_apply();
    let err = config
        .resolve_from_secret_manager()
        .await
        .expect_err("missing secret id");
    assert!(err.contains("NVBES_SECRET_MANAGER_SECRET_ID"));
}

#[tokio::test]
async fn resolve_fetches_reports_unavailable_secret_manager() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("ephemeral bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    let base = format!("http://{addr}");
    let _env = SecretManagerEnv::install(&[
        ("NVBES_SECRET_MANAGER_SECRET_ID", Some("test-secret")),
        ("NVBES_SECRET_MANAGER_REGION", Some("nl-ams")),
        ("NVBES_SECRET_MANAGER_AUTH_TOKEN", Some("token")),
        ("NVBES_SECRET_MANAGER_API_BASE_URL", Some(base.as_str())),
    ]);
    let mut config = development_config_ready_for_secret_apply();
    let err = config
        .resolve_from_secret_manager()
        .await
        .expect_err("unavailable");
    assert!(err.contains("Secret Manager is unavailable"), "{err}");
}

#[tokio::test]
async fn resolve_fetches_rejects_non_success_status() {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        stream
            .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n")
            .await
            .expect("write");
    });
    let base = format!("http://{addr}");
    let _env = SecretManagerEnv::install(&[
        ("NVBES_SECRET_MANAGER_SECRET_ID", Some("test-secret")),
        ("NVBES_SECRET_MANAGER_REGION", Some("pl-waw")),
        ("NVBES_SECRET_MANAGER_AUTH_TOKEN", Some("token")),
        ("NVBES_SECRET_MANAGER_API_BASE_URL", Some(base.as_str())),
    ]);
    let mut config = development_config_ready_for_secret_apply();
    let err = config
        .resolve_from_secret_manager()
        .await
        .expect_err("rejected");
    assert!(err.contains("Secret Manager rejected access"), "{err}");
    server.await.expect("server");
}

#[tokio::test]
async fn resolve_fetches_rejects_invalid_json_payload() {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let body = b"not-json";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        stream.write_all(response.as_bytes()).await.expect("write");
    });
    let base = format!("http://{addr}");
    let _env = SecretManagerEnv::install(&[
        ("NVBES_SECRET_MANAGER_SECRET_ID", Some("test-secret")),
        ("NVBES_SECRET_MANAGER_REGION", Some("fr-par")),
        ("NVBES_SECRET_MANAGER_AUTH_TOKEN", Some("token")),
        ("NVBES_SECRET_MANAGER_API_BASE_URL", Some(base.as_str())),
    ]);
    let mut config = development_config_ready_for_secret_apply();
    let err = config
        .resolve_from_secret_manager()
        .await
        .expect_err("invalid json");
    assert!(err.contains("invalid JSON"), "{err}");
    server.await.expect("server");
}

#[tokio::test]
async fn resolve_fetches_applies_secret_payload_from_loopback() {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    let stripe_key = format!("{}_{}", "sk_test", "from_secret_manager");
    let secrets = json!({
        "database_url": "postgres://postgres:postgres@127.0.0.1:5432/nvbes",
        "billing_database_url": "postgres://postgres:postgres@127.0.0.1:5432/billing",
        "jwt_secret": "secret-manager-jwt-with-32-chars!!",
        "stripe_secret_key": stripe_key,
    });
    let encoded = base64::engine::general_purpose::STANDARD.encode(secrets.to_string());
    let payload = json!({ "data": encoded }).to_string();

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{payload}",
            payload.len()
        );
        stream.write_all(response.as_bytes()).await.expect("write");
    });
    let base = format!("http://{addr}");
    let _env = SecretManagerEnv::install(&[
        ("NVBES_SECRET_MANAGER_SECRET_ID", Some("test-secret")),
        ("NVBES_SECRET_MANAGER_REGION", Some("fr-par")),
        ("NVBES_SECRET_MANAGER_AUTH_TOKEN", Some("token")),
        ("NVBES_SECRET_MANAGER_API_BASE_URL", Some(base.as_str())),
    ]);
    let mut config = development_config_ready_for_secret_apply();
    config
        .resolve_from_secret_manager()
        .await
        .expect("secret manager resolve");
    assert_eq!(
        config.database_url,
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes"
    );
    assert_eq!(config.jwt_secret, "secret-manager-jwt-with-32-chars!!");
    let expected_stripe = format!("{}_{}", "sk_test", "from_secret_manager");
    assert_eq!(
        config.stripe_secret_key.as_deref(),
        Some(expected_stripe.as_str())
    );
    server.await.expect("server");
}
