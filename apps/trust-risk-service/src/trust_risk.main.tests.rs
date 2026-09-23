use std::ffi::OsString;

use tokio::sync::{Mutex, watch};

use super::{run, server_shutdown};

const TEST_VARS: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_TRUST_RISK_DATABASE_URL",
    "NVBES_TRUST_RISK_BIND_ADDR",
    "NVBES_TRUST_RISK_PRODUCER_POLICIES",
    "NVBES_TRUST_RISK_OPERATOR_TOKENS",
    "NVBES_TRUST_RISK_METRICS_TOKEN",
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
    let err = run(vec!["not-a-command".into()]).await.unwrap_err();
    assert!(err.to_string().contains("usage:"));
}

#[tokio::test]
async fn run_validate_runtime_checks_lazy_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set(
        "NVBES_TRUST_RISK_DATABASE_URL",
        "postgres://localhost/nvbes_trust_risk",
    );
    run(vec!["validate-runtime".into()])
        .await
        .expect("validate-runtime");
}

#[tokio::test]
async fn run_error_reporting_smoke_prints_json() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set(
        "NVBES_TRUST_RISK_DATABASE_URL",
        "postgres://localhost/nvbes_trust_risk",
    );
    run(vec!["error-reporting-smoke".into()])
        .await
        .expect("smoke");
}

#[tokio::test]
async fn run_erase_subject_rejects_non_integer_kind() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set(
        "NVBES_TRUST_RISK_DATABASE_URL",
        "postgres://localhost/nvbes_trust_risk",
    );
    let err = run(vec![
        "erase-subject".into(),
        "not-int".into(),
        "nvbes.identity".into(),
        "principal:018f7f2d-fc7d".into(),
        "operator".into(),
        "valid reason".into(),
    ])
    .await
    .unwrap_err();
    assert!(err.to_string().contains("subject kind must be an integer"));
}

#[tokio::test]
async fn run_without_arguments_starts_runtime_until_stopped() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    guard.set("NVBES_ENVIRONMENT", "development");
    guard.set("NVBES_TRUST_RISK_BIND_ADDR", "127.0.0.1:0");
    guard.set(
        "NVBES_TRUST_RISK_DATABASE_URL",
        "postgres://localhost/nvbes_trust_risk",
    );
    let handle = tokio::spawn(run(vec![]));
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    handle.abort();
}

#[cfg(feature = "database-tests")]
mod database {
    use std::time::Duration;

    use axum::body::Body;
    use tokio::sync::watch;
    use tower::ServiceExt;

    use super::{ENV_LOCK, EnvGuard};
    use crate::{app::TrustRiskState, build_router, database, grpc_test_support};

    fn database_url() -> String {
        std::env::var("DATABASE_URL").expect("DATABASE_URL for sqlx::test")
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn build_router_exposes_live_health(pool: sqlx::PgPool) {
        let state = grpc_test_support::state(pool);
        let app = build_router(state).await.expect("router");
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn serve_stops_background_workers_without_signals(_pool: sqlx::PgPool) {
        let _lock = ENV_LOCK.lock().await;
        let guard = EnvGuard::isolated();
        guard.set("NVBES_ENVIRONMENT", "test");
        guard.set("NVBES_TRUST_RISK_BIND_ADDR", "127.0.0.1:0");
        guard.set("NVBES_TRUST_RISK_DATABASE_URL", database_url());

        let config = crate::config::TrustRiskConfig::from_env().expect("config");
        let db = database::connect_lazy(&config.database_url).expect("lazy pool");
        let state = TrustRiskState::new(config, db);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let router = build_router(state.clone()).await.expect("router");
        let projection_task =
            tokio::spawn(crate::projection::run(state.clone(), shutdown_rx.clone()));
        let retention_task =
            tokio::spawn(crate::retention::run(state.clone(), shutdown_rx.clone()));
        let server = axum::serve(listener, router)
            .with_graceful_shutdown(super::server_shutdown(shutdown_rx.clone()));
        let server_task = tokio::spawn(async move { server.await });

        tokio::time::sleep(Duration::from_millis(300)).await;
        shutdown_tx.send(true).unwrap();
        projection_task.await.unwrap();
        retention_task.await.unwrap();
        server_task.await.unwrap().unwrap();
    }
}
