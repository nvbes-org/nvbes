use std::ffi::OsString;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use super::{run, run_deployment_bootstrap, server_shutdown};

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_BILLING_DATABASE_URL",
    "DATABASE_URL",
    "NVBES_BILLING_BIND_ADDR",
    "NVBES_BILLING_HTTP_BIND_ADDR",
    "NVBES_APP_URL",
    "NVBES_STRIPE_SECRET_KEY",
    "STRIPE_SECRET_KEY",
    "NVBES_EMAIL_GRPC_ENDPOINT",
    "NVBES_EMAIL_PRODUCER_TOKEN",
    "NVBES_EMAIL_TOKEN",
    "NVBES_BILLING_GRPC_ENDPOINT",
    "NVBES_BILLING_GRPC_AUTH_TOKEN",
    "NVBES_BILLING_METRICS_TOKEN",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
    "NVBES_BILLING_QUEUE_ENDPOINT",
    "NVBES_BILLING_QUEUE_URL",
    "NVBES_BILLING_QUEUE_REGION",
    "NVBES_BILLING_QUEUE_ACCESS_KEY",
    "NVBES_BILLING_QUEUE_SECRET_KEY",
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
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
        }),
    );
    guard.set("NVBES_APP_URL", "https://nvbes.test");
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        Duration::from_millis(200),
    )
    .is_ok()
}

#[tokio::test]
async fn server_shutdown_completes_when_watch_channel_closes() {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    drop(sender);
    server_shutdown(receiver).await;
}

#[tokio::test]
async fn server_shutdown_completes_when_shutdown_flag_is_set() {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    let waiting = tokio::spawn(server_shutdown(receiver));
    sender.send(true).unwrap();
    waiting.await.unwrap();
}

#[tokio::test]
async fn run_rejects_unknown_commands() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["not-a-command".into()]).await.unwrap_err();
    assert!(err.to_string().contains("usage: nvbes-billing-worker"));
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
    assert!(err.to_string().contains("Live Stripe credentials"), "{err}");
}

#[tokio::test]
async fn deployment_bootstrap_rejects_invalid_bind_addr() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", "not-a-socket-addr");
    let err = run_deployment_bootstrap().await.unwrap_err();
    assert!(
        err.to_string()
            .contains("NVBES_BILLING_HTTP_BIND_ADDR is invalid"),
        "{err}"
    );
}

#[tokio::test]
async fn deployment_bootstrap_serves_live_health_check() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = probe.local_addr().expect("local addr").to_string();
    drop(probe);
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", &addr);

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
async fn deployment_bootstrap_accepts_legacy_bind_addr_env() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = probe.local_addr().expect("local addr").to_string();
    drop(probe);
    guard.set("NVBES_BILLING_BIND_ADDR", &addr);

    let handle = tokio::spawn(run(vec!["deployment-bootstrap".into()]));
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

#[tokio::test]
async fn run_synthetic_smoke_fails_for_unreachable_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["synthetic-smoke".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_synthetic_smoke_succeeds_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping synthetic-smoke integration: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    run(vec!["synthetic-smoke".into()])
        .await
        .expect("synthetic-smoke");
}

#[tokio::test]
async fn serve_stops_background_workers_without_process_signals() {
    if !postgres_reachable() {
        eprintln!("skipping serve shutdown smoke: postgres unavailable");
        return;
    }

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", "127.0.0.1:0");

    let config = crate::config::BillingWorkerConfig::from_env().expect("config");
    let db = crate::database::connect(&config.database_url, 2)
        .await
        .expect("connect");
    let state = crate::state::BillingWorkerState::new(config, db.clone())
        .await
        .expect("state");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let serve = tokio::spawn(super::serve(state, db, listener, async move {
        let _ = stop_rx.await;
    }));
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stop_tx.send(());
    serve.await.expect("join").expect("serve");
}

#[tokio::test]
async fn serve_without_local_dispatcher_shuts_down_cleanly() {
    if !postgres_reachable() {
        eprintln!("skipping scaleway serve shutdown smoke: postgres unavailable");
        return;
    }

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);

    let mut config = crate::config::BillingWorkerConfig::from_env().expect("config");
    config.dispatch_mode =
        crate::config::DispatchMode::Scaleway(crate::config::DispatchQueueConfig {
            endpoint: "https://sqs.example.test".into(),
            queue_url: "https://sqs.example.test/billing".into(),
            region: "fr-par".into(),
            access_key: "SCWTEST".into(),
            secret_key: "secret".into(),
        });
    let db = crate::database::connect(&config.database_url, 2)
        .await
        .expect("connect");
    let state = crate::state::BillingWorkerState::new(config, db.clone())
        .await
        .expect("state");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let serve = tokio::spawn(super::serve(state, db, listener, async move {
        let _ = stop_rx.await;
    }));
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stop_tx.send(());
    serve.await.expect("join").expect("serve");
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
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", "127.0.0.1:0");
    let handle = tokio::spawn(run(vec![]));
    tokio::time::sleep(Duration::from_millis(400)).await;
    handle.abort();
    let _ = handle.await;
}

#[tokio::test]
async fn run_serve_alias_starts_runtime_until_stopped() {
    if !postgres_reachable() {
        eprintln!("skipping serve alias smoke: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", "127.0.0.1:0");
    let handle = tokio::spawn(run(vec!["serve".into()]));
    tokio::time::sleep(Duration::from_millis(400)).await;
    handle.abort();
    let _ = handle.await;
}
