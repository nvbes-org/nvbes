use tokio::sync::watch;

use crate::{config::BillingWorkerConfig, state::BillingWorkerState};

use super::{DispatchOutcome, dispatch_message, run_local};

async fn lazy_state() -> BillingWorkerState {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://127.0.0.1:1/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "http://localhost:3000".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };

    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://127.0.0.1:1/unused")
        .unwrap();

    BillingWorkerState::new(config, db).await.unwrap()
}

#[tokio::test]
async fn dispatch_message_acknowledges_empty_identifier() {
    let state = lazy_state().await;
    let outcome = dispatch_message(&state, "   ")
        .await
        .expect("empty identifier must not touch the database");
    assert_eq!(outcome, DispatchOutcome::Acknowledged);
}

#[tokio::test]
async fn run_local_stops_on_shutdown_signal() {
    let state = lazy_state().await;
    let (_tx, rx) = tokio::sync::mpsc::channel::<String>(1);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let worker = tokio::spawn(run_local(state, rx, shutdown_rx));
    // Give the loop a chance to start, then stop before the 30s sweep interval.
    tokio::task::yield_now().await;
    shutdown_tx.send(true).expect("signal shutdown");
    tokio::time::timeout(std::time::Duration::from_secs(2), worker)
        .await
        .expect("local loop must exit promptly")
        .expect("local loop must exit cleanly");
}
