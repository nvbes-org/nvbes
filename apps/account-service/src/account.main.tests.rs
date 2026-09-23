use std::ffi::OsString;
use std::sync::MutexGuard;
use std::time::Duration;

use super::{required_uuid, run, serve};
use crate::test_support::test_env_lock;

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_ACCOUNT_DATABASE_URL",
    "NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS",
    "NVBES_ACCOUNT_BIND_ADDR",
    "PORT",
    "NVBES_IDENTITY_TOKEN_ISSUER",
    "NVBES_ACCOUNT_TOKEN_AUDIENCE",
    "NVBES_IDENTITY_TOKEN_KEY_ID",
    "NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM",
    "NVBES_ACCOUNT_METRICS_TOKEN",
    "NVBES_ACCOUNT_SYNTHETIC_OWNER_ID",
    "NVBES_ACCOUNT_SYNTHETIC_MEMBER_ID",
    "SENTRY_DSN",
    "SENTRY_TRACES_SAMPLE_RATE",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
];

const DEV_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwEKbtGra6bWscKp5s8i3\njvE0mUrZV54vaDWEfYxnjJYc6TNPBUuHlNUOv44eePZ+TxLQ9wPYSwuhSJIPAX7c\nAkDDdzVJy36lUVTjKGND7PZtgv6ItPQb6yM7YNyM7+QHZXL0fvB5q7O1AasgT+sF\ns0ffePjSyi9QpOww8TqcgePyXN3anUmB8pwoaJQfJOOLE1sJ3zsfw+n3nG+31Lpg\nDQBwYQWrKRRH/R7aYRFoZu1ZRFINfYXVmOGUuehcYkAprNs1dte6szKyyc2zhUJi\nTUez2NsFzgzx1Q1ssxYOMbQcVqNjux5l2aC9i5VcPi7gRHWGKiN77sSjnz90vqSu\nYwIDAQAB\n-----END PUBLIC KEY-----\n";

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
        "NVBES_ACCOUNT_DATABASE_URL",
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
        }),
    );
    guard.set("NVBES_IDENTITY_TOKEN_KEY_ID", "identity-key-1");
    guard.set("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM", DEV_PUBLIC_KEY);
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
    assert!(err.to_string().contains("usage: nvbes-account-service"));
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
async fn required_uuid_reports_missing_and_invalid_values() {
    let guard = EnvGuard::isolated();
    let missing = required_uuid("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID").unwrap_err();
    assert!(missing.to_string().contains("is required"));

    guard.set("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID", "not-a-uuid");
    let invalid = required_uuid("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID").unwrap_err();
    assert!(!invalid.to_string().is_empty());

    let id = uuid::Uuid::new_v4();
    guard.set("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID", id.to_string());
    assert_eq!(
        required_uuid("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID").expect("parse"),
        id
    );
}

#[tokio::test]
async fn run_synthetic_account_smoke_requires_owner_uuid() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["synthetic-account-smoke".into()])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID"));
}

#[tokio::test]
async fn run_process_privacy_jobs_fails_for_unreachable_database() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_ACCOUNT_DATABASE_URL", "postgres://invalid");
    let err = run(vec!["process-privacy-jobs".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_migrate_fails_for_unreachable_database() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_ACCOUNT_DATABASE_URL", "postgres://invalid");
    let err = run(vec!["migrate".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_migrate_applies_schema_when_database_is_available() {
    use std::process::Command;

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
        "account_main_{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let disposable_url = match admin.rfind('/') {
        Some(idx) => format!("{}{db_name}", &admin[..=idx]),
        None => {
            eprintln!("skipping migrate integration: invalid DATABASE_URL");
            return;
        }
    };

    let drop = Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("DROP DATABASE IF EXISTS {db_name}"),
        ])
        .output()
        .expect("drop disposable database");
    if !drop.status.success() {
        eprintln!(
            "skipping migrate integration: {}",
            String::from_utf8_lossy(&drop.stderr)
        );
        return;
    }

    let create = Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE DATABASE {db_name} TEMPLATE template0"),
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

    guard.set("NVBES_ACCOUNT_DATABASE_URL", &disposable_url);
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
async fn run_without_arguments_starts_runtime_until_stopped() {
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_ACCOUNT_BIND_ADDR", "127.0.0.1:0");
    let handle = tokio::spawn(run(vec![]));
    tokio::time::sleep(Duration::from_millis(400)).await;
    handle.abort();
    let _ = handle.await;
}

#[tokio::test]
async fn serve_exposes_live_health_until_stopped() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_ACCOUNT_BIND_ADDR", "127.0.0.1:0");

    let config = crate::config::AccountConfig::from_env().expect("config");
    let db = crate::database::connect_lazy(&config.database_url, config.database_max_connections)
        .expect("lazy pool");
    let state = crate::app::AccountState::new(config, db.clone()).expect("state");
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
