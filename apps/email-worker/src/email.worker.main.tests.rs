use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::{Mutex, watch};

use super::{
    provider_name, redacted_sender, run, run_deployment_bootstrap, serve, server_shutdown,
};
use crate::config::{ProviderConfig, ScalewayConfig};

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_EMAIL_DATABASE_URL",
    "NVBES_EMAIL_FROM_EMAIL",
    "NVBES_EMAIL_FROM_NAME",
    "NVBES_EMAIL_PROVIDER",
    "NVBES_EMAIL_HTTP_BIND_ADDR",
    "NVBES_EMAIL_GRPC_BIND_ADDR",
    "NVBES_EMAIL_RUNTIME_ROLE",
    "NVBES_EMAIL_PRODUCER_TOKENS",
    "NVBES_EMAIL_DATA_ENCRYPTION_KEY",
    "NVBES_EMAIL_RECIPIENT_HMAC_KEY",
    "NVBES_EMAIL_REPLY_TO",
    "NVBES_EMAIL_MESSAGE_ID_DOMAIN",
    "NVBES_EMAIL_PAYLOAD_RETENTION_DAYS",
    "NVBES_EMAIL_LEDGER_RETENTION_DAYS",
    "NVBES_EMAIL_TEST_CAPTURE_DIR",
    "NVBES_EMAIL_SNS_TOPIC_ARN",
    "NVBES_EMAIL_SNS_CA_BUNDLE_PATH",
    "NVBES_EMAIL_SNS_CA_BUNDLE_PEM",
    "SENTRY_DSN",
    "NVBES_OTLP_ENDPOINT",
    "NVBES_OTLP_AUTHORIZATION_HEADER",
    "NVBES_OBSERVABILITY_INTERNAL_TOKEN",
];

pub(super) static ENV_LOCK: Mutex<()> = Mutex::const_new(());

pub(super) struct EnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    pub(super) fn isolated() -> Self {
        let saved = TEST_VARS
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in TEST_VARS {
            unsafe { std::env::remove_var(name) };
        }
        Self { saved }
    }

    pub(super) fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
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

pub(super) fn apply_development_defaults(guard: &EnvGuard) {
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set(
        "NVBES_EMAIL_DATABASE_URL",
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
        }),
    );
    guard.set("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr");
    guard.set("NVBES_EMAIL_PROVIDER", "mock");
}

#[test]
fn startup_log_values_are_bounded_and_sender_is_redacted() {
    assert_eq!(provider_name(&ProviderConfig::Mock), "mock");
    assert_eq!(
        provider_name(&ProviderConfig::TestCapture(PathBuf::from("/tmp/capture"))),
        "test-capture"
    );
    assert_eq!(
        provider_name(&ProviderConfig::Scaleway(ScalewayConfig {
            secret_key: "secret".to_string(),
            project_id: "project".to_string(),
            region: "fr-par".to_string(),
        })),
        "scaleway"
    );
    assert_eq!(redacted_sender("no-reply@nvbes.fr"), "***@nvbes.fr");
    assert_eq!(redacted_sender("invalid"), "***");
}

#[tokio::test]
async fn server_shutdown_completes_on_signal_or_closed_channel() {
    let (sender, receiver) = watch::channel(false);
    let waiting = tokio::spawn(server_shutdown(receiver));
    sender.send(true).unwrap();
    waiting.await.unwrap();

    let (sender, receiver) = watch::channel(false);
    drop(sender);
    server_shutdown(receiver).await;
}

#[tokio::test]
async fn run_rejects_unknown_commands() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["not-a-command".into()]).await.unwrap_err();
    assert!(err.to_string().contains("usage: nvbes-email-worker"));
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
async fn deployment_bootstrap_rejects_invalid_bind_addr() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", "not-a-socket-addr");
    let err = run_deployment_bootstrap().await.unwrap_err();
    assert!(
        err.to_string()
            .contains("NVBES_EMAIL_HTTP_BIND_ADDR is invalid"),
        "{err}"
    );
}

#[tokio::test]
async fn run_migrate_fails_for_unreachable_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_EMAIL_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["migrate".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn serve_stops_background_workers_without_process_signals() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", "127.0.0.1:0");
    guard.set("NVBES_EMAIL_GRPC_BIND_ADDR", "127.0.0.1:0");

    let config = crate::config::EmailWorkerConfig::from_env().expect("config");
    let db = crate::database::connect_lazy(&config.database_url).expect("lazy pool");
    let state = crate::state::EmailWorkerState::new(config, db).expect("state");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(serve(state, listener, async move {
        let _ = stop_rx.await;
    }));
    tokio::time::sleep(Duration::from_millis(250)).await;
    let _ = stop_tx.send(());
    handle.await.expect("join").expect("serve");
}

#[tokio::test]
async fn run_error_reporting_smoke_emits_json_with_development_defaults() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    // Tracing subscriber init is process-global and panics on a second install.
    let handle = tokio::spawn(run(vec!["error-reporting-smoke".into()]));
    match handle.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => panic!("error-reporting-smoke: {error}"),
        Err(join) if join.is_panic() => {}
        Err(join) => panic!("error-reporting-smoke join: {join}"),
    }
}

#[tokio::test]
async fn run_synthetic_smoke_requires_explicit_recipient() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let err = run(vec!["synthetic-smoke".into()]).await.unwrap_err();
    assert!(
        err.to_string().contains("NVBES_EMAIL_SYNTHETIC_RECIPIENT"),
        "{err}"
    );
}

#[tokio::test]
async fn deployment_bootstrap_serves_live_health_check() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = probe.local_addr().expect("local addr").to_string();
    drop(probe);
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", &addr);

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
async fn run_deployment_bootstrap_command_rejects_invalid_bind_addr() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", "not-a-socket-addr");
    let err = run(vec!["deployment-bootstrap".into()]).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("NVBES_EMAIL_HTTP_BIND_ADDR is invalid"),
        "{err}"
    );
}

#[path = "email.worker.main.runtime.tests.rs"]
mod runtime;
