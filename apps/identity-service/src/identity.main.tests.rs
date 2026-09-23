use std::ffi::OsString;
use std::process::Command;
use std::time::Duration;

use std::sync::MutexGuard;

use super::{required_secret, run, serve};
use crate::database::database_test_support::test_env_lock;

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_IDENTITY_DATABASE_URL",
    "NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS",
    "NVBES_IDENTITY_BIND_ADDR",
    "PORT",
    "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
    "NVBES_IDENTITY_MFA_KEY_VERSION",
    "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY",
    "NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION",
    "NVBES_IDENTITY_METRICS_TOKEN",
    "SENTRY_DSN",
    "SENTRY_TRACES_SAMPLE_RATE",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
    "NVBES_IDENTITY_SYNTHETIC_EMAIL",
    "NVBES_IDENTITY_SYNTHETIC_PASSWORD",
    "NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD",
    "NVBES_IDENTITY_SYNTHETIC_TOKEN_AUDIENCE",
    "NVBES_IDENTITY_RECOVERY_BASE_URL",
];

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn isolated() -> Self {
        let lock = test_env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = TEST_VARS
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in TEST_VARS {
            // SAFETY: serialized by `test_env_lock` for test-only env mutation.
            unsafe { std::env::remove_var(name) };
        }
        Self { _lock: lock, saved }
    }

    fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        // SAFETY: caller holds `test_env_lock` via `EnvGuard`.
        unsafe { std::env::set_var(name, value) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, val) in &self.saved {
            match val {
                // SAFETY: serialized by `test_env_lock` for test-only env mutation.
                Some(v) => unsafe { std::env::set_var(name, v) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

fn apply_development_defaults(guard: &EnvGuard) {
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set(
        "NVBES_IDENTITY_DATABASE_URL",
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
        }),
    );
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        Duration::from_millis(200),
    )
    .is_ok()
}

#[tokio::test]
async fn run_rejects_unknown_commands() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["not-a-command".into()]).await.unwrap_err();
    assert!(err.to_string().contains("usage: nvbes-identity-service"));
}

#[tokio::test]
async fn run_validate_runtime_accepts_development_defaults() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    run(vec!["validate-runtime".into()])
        .await
        .expect("validate-runtime");
}

#[tokio::test]
async fn required_secret_reports_missing_and_blank_values() {
    let guard = EnvGuard::isolated();
    let missing = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL").unwrap_err();
    assert!(missing.to_string().contains("is required"));

    guard.set("NVBES_IDENTITY_SYNTHETIC_EMAIL", "   ");
    let blank = required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL").unwrap_err();
    assert!(blank.to_string().contains("is required"));

    guard.set("NVBES_IDENTITY_SYNTHETIC_EMAIL", "coverage@nvbes.test");
    assert_eq!(
        required_secret("NVBES_IDENTITY_SYNTHETIC_EMAIL").expect("secret"),
        "coverage@nvbes.test"
    );
}

#[tokio::test]
async fn run_synthetic_auth_smoke_requires_explicit_credentials() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["synthetic-auth-smoke".into()]).await.unwrap_err();
    assert!(err.to_string().contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"));
}

#[tokio::test]
async fn run_synthetic_mfa_smoke_requires_explicit_credentials() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["synthetic-mfa-smoke".into()]).await.unwrap_err();
    assert!(err.to_string().contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"));
}

#[tokio::test]
async fn run_synthetic_token_smoke_requires_explicit_credentials() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["synthetic-token-smoke".into()]).await.unwrap_err();
    assert!(err.to_string().contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"));
}

#[tokio::test]
async fn run_synthetic_auth_email_smoke_requires_explicit_credentials() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["synthetic-auth-email-smoke".into()])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"));
}

#[tokio::test]
async fn run_error_reporting_smoke_requires_sentry_configuration() {
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "development");
    let err = run(vec!["error-reporting-smoke".into()]).await.unwrap_err();
    assert!(err.to_string().contains("SENTRY_DSN"), "{err}");
}

#[tokio::test]
async fn run_migrate_fails_for_unreachable_database() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_IDENTITY_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["migrate".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_migrate_applies_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }

    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);

    let admin = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
    });
    let db_name = format!(
        "identity_main_{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let disposable_url = match admin.rfind('/') {
        Some(idx) => format!("{}{db_name}", &admin[..=idx]),
        None => {
            eprintln!("skipping migrate integration: invalid DATABASE_URL");
            return;
        }
    };

    let create = Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE DATABASE {db_name}"),
        ])
        .output()
        .expect("create disposable database");
    if !create.status.success() {
        eprintln!(
            "skipping migrate integration: {}",
            String::from_utf8_lossy(&create.stderr)
        );
        return;
    }

    guard.set("NVBES_IDENTITY_DATABASE_URL", &disposable_url);
    let migrate_result = run(vec!["migrate".into()]).await;

    let _ = Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("DROP DATABASE IF EXISTS {db_name}"),
        ])
        .output();

    migrate_result.expect("migrate");
}

#[tokio::test]
async fn run_rotate_mfa_key_fails_for_unreachable_database() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_IDENTITY_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["rotate-mfa-key".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_without_arguments_starts_runtime_until_stopped() {
    if !postgres_reachable() {
        eprintln!("skipping serve smoke: postgres unavailable");
        return;
    }
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_IDENTITY_BIND_ADDR", "127.0.0.1:0");
    let handle = tokio::spawn(run(vec![]));
    tokio::time::sleep(Duration::from_millis(400)).await;
    handle.abort();
    let _ = handle.await;
}

#[tokio::test]
async fn serve_exposes_live_health_until_stopped() {
    if !postgres_reachable() {
        eprintln!("skipping serve health smoke: postgres unavailable");
        return;
    }

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_IDENTITY_BIND_ADDR", "127.0.0.1:0");

    let config = crate::config::IdentityConfig::from_env().expect("config");
    let db = crate::database::connect_lazy(&config.database_url, config.database_max_connections)
        .expect("lazy pool");
    let state = crate::app::IdentityState::new(config, db.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr").to_string();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(serve(state, db, listener, async move {
        let _ = stop_rx.await;
    }));
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
            let mut buf = [0_u8; 256];
            if let Ok(read) = stream.read(&mut buf).await
                && read > 0
            {
                response = Some(String::from_utf8_lossy(&buf[..read]).to_string());
                break;
            }
        }
    }

    let _ = stop_tx.send(());
    handle.await.expect("join").expect("serve");
    let body = response.expect("health response");
    assert!(body.contains("200") || body.contains("alive"), "{body}");
}
