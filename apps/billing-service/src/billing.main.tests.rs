use std::ffi::OsString;

use tokio::sync::Mutex;

use super::{required_uuid, run, run_deployment_bootstrap};

const TEST_VARS: &[&str] = &[
    "NVBES_BILLING_DATABASE_URL",
    "NVBES_BILLING_BIND_ADDR",
    "NVBES_BILLING_HTTP_BIND_ADDR",
    "NVBES_STRIPE_SECRET_KEY",
    "NVBES_STRIPE_WEBHOOK_SECRET",
    "NVBES_APP_URL",
    "NVBES_IDENTITY_PUBLIC_KEY_PEM",
    "NVBES_BILLING_OPERATOR_TOKEN",
    "NVBES_BILLING_METRICS_TOKEN",
    "NVBES_EMAIL_GRPC_ENDPOINT",
    "NVBES_BILLING_EMAIL_TOKEN",
    "NVBES_EMAIL_INTERNAL_TOKEN",
    "NVBES_BILLING_SYNTHETIC_WORKSPACE_ID",
    "NVBES_BILLING_SYNTHETIC_OWNER_ID",
];

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

struct EnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn isolated() -> Self {
        let saved = TEST_VARS
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in TEST_VARS {
            unsafe { std::env::remove_var(name) };
        }
        Self { saved }
    }

    fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe { std::env::set_var(name, value) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, val) in &self.saved {
            match val {
                Some(v) => unsafe { std::env::set_var(name, v) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

fn apply_development_defaults(guard: &EnvGuard) {
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/nvbes_billing".into()),
    );
    guard.set("NVBES_STRIPE_SECRET_KEY", "sk_test_dummy_key_for_testing");
    guard.set("NVBES_STRIPE_WEBHOOK_SECRET", "whsec_test_dummy_secret");
    guard.set("NVBES_APP_URL", "https://nvbes.test");
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}

#[tokio::test]
async fn run_rejects_unknown_commands() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["not-a-command".into()]).await.unwrap_err();
    assert!(err.to_string().contains("usage: nvbes-billing-service"));
}

#[tokio::test]
async fn run_validate_runtime_accepts_development_defaults() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    run(vec!["validate-runtime".into()])
        .await
        .expect("validate-runtime");
}

#[tokio::test]
async fn run_rejects_live_stripe_credentials() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_STRIPE_SECRET_KEY",
        format!("{}_{}", "sk_live", "forbidden_in_v1"),
    );
    let err = run(vec!["validate-runtime".into()]).await.unwrap_err();
    assert!(err.to_string().contains("FINOPS"), "{err}");
}

#[tokio::test]
async fn required_uuid_reports_missing_and_invalid_values() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    let missing = required_uuid("NVBES_BILLING_SYNTHETIC_WORKSPACE_ID").unwrap_err();
    assert!(missing.to_string().contains("is required"));

    guard.set("NVBES_BILLING_SYNTHETIC_WORKSPACE_ID", "not-a-uuid");
    let invalid = required_uuid("NVBES_BILLING_SYNTHETIC_WORKSPACE_ID").unwrap_err();
    assert!(!invalid.to_string().is_empty());

    let id = uuid::Uuid::new_v4();
    guard.set("NVBES_BILLING_SYNTHETIC_WORKSPACE_ID", id.to_string());
    assert_eq!(
        required_uuid("NVBES_BILLING_SYNTHETIC_WORKSPACE_ID").expect("parse"),
        id
    );
}

#[tokio::test]
async fn deployment_bootstrap_rejects_invalid_bind_addr() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_BIND_ADDR", "not-a-socket-addr");
    let err = run_deployment_bootstrap().await.unwrap_err();
    assert!(
        err.to_string()
            .contains("NVBES_BILLING_BIND_ADDR is invalid"),
        "{err}"
    );
}

#[tokio::test]
async fn deployment_bootstrap_serves_live_health_check() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = probe.local_addr().expect("local addr").to_string();
    drop(probe);
    guard.set("NVBES_BILLING_BIND_ADDR", &addr);

    let handle = tokio::spawn(run_deployment_bootstrap());
    let mut response = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if handle.is_finished() {
            break;
        }
        if let Ok(mut stream) = tokio::net::TcpStream::connect(&addr).await {
            if stream
                .write_all(b"GET /health/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .is_err()
            {
                continue;
            }
            let mut buf = [0_u8; 128];
            if let Ok(read) = stream.read(&mut buf).await
                && read > 0
            {
                response = Some(String::from_utf8_lossy(&buf[..read]).to_string());
                break;
            }
        }
    }

    handle.abort();
    let _ = handle.await;
    let body = response.expect("health response");
    assert!(body.contains("204") || body.contains("200"), "{body}");
}

#[tokio::test]
async fn run_migrate_applies_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    run(vec!["migrate".into()]).await.expect("migrate");
}

#[tokio::test]
async fn run_without_arguments_starts_runtime_until_stopped() {
    if !postgres_reachable() {
        eprintln!("skipping serve smoke: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_BILLING_BIND_ADDR", "127.0.0.1:0");
    let handle = tokio::spawn(run(vec![]));
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    handle.abort();
}

#[tokio::test]
async fn run_migrate_fails_for_unreachable_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["migrate".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}
