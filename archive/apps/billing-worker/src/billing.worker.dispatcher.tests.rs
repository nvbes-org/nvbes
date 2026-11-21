use chrono::Utc;
use serde_json::json;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{config::BillingWorkerConfig, state::BillingWorkerState};

use super::{
    DispatchOutcome, OutboxRow, dispatch_message, failure_notification_from_event,
    receipt_notification_from_event, run_local,
};

async fn lazy_state() -> BillingWorkerState {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
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
        .connect_lazy("postgres://localhost/unused")
        .unwrap();

    BillingWorkerState::new(config, db).await.unwrap()
}

fn sample_row(event_type: &str, payload: serde_json::Value) -> OutboxRow {
    OutboxRow {
        id: 1,
        event_uuid: Uuid::new_v4(),
        event_type: event_type.into(),
        aggregate_id: Uuid::new_v4(),
        payload,
        published_at: None,
        created_at: Utc::now(),
    }
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
async fn dispatch_message_acknowledges_unknown_identifier_format() {
    let state = lazy_state().await;
    let outcome = dispatch_message(&state, "not-a-uuid-or-id")
        .await
        .expect("unknown format must ack");
    assert_eq!(outcome, DispatchOutcome::Acknowledged);
}

#[tokio::test]
async fn run_local_stops_on_shutdown_signal() {
    let state = lazy_state().await;
    state.db.close().await;

    let (_tx, rx) = tokio::sync::mpsc::channel::<String>(1);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let worker = tokio::spawn(run_local(state, rx, shutdown_rx));
    tokio::task::yield_now().await;
    shutdown_tx.send(true).expect("signal shutdown");
    tokio::time::timeout(std::time::Duration::from_secs(2), worker)
        .await
        .expect("local loop must exit promptly")
        .expect("local loop must exit cleanly");
}

#[test]
fn receipt_notification_requires_recipient_and_defaults_fields() {
    let row = sample_row(
        "billing.invoice.paid.v1",
        json!({
            "recipient_email": "alice@example.test",
            "customer_name": "Alice",
            "amount_minor": 4200,
            "currency": "USD",
            "invoice_id": "in_42",
            "invoice_url": "https://stripe.test/in_42"
        }),
    );
    let notif = receipt_notification_from_event(&row).expect("recipient present");
    assert_eq!(notif.recipient_email, "alice@example.test");
    assert_eq!(notif.amount_minor, 4200);
    assert_eq!(notif.currency, "USD");
    assert_eq!(notif.invoice_id, "in_42");

    let sparse = sample_row(
        "billing.invoice.paid.v1",
        json!({ "recipient_email": "bob@example.test" }),
    );
    let notif = receipt_notification_from_event(&sparse).expect("defaults");
    assert_eq!(notif.amount_minor, 0);
    assert_eq!(notif.currency, "EUR");
    assert_eq!(notif.invoice_id, "in_unknown");
    assert!(receipt_notification_from_event(&sample_row("x", json!({}))).is_none());
}

#[test]
fn failure_notification_preserves_optional_invoice_url() {
    let row = sample_row(
        "billing.invoice.payment_failed.v1",
        json!({
            "recipient_email": "carol@example.test",
            "invoice_id": "in_fail",
            "invoice_url": "https://stripe.test/in_fail"
        }),
    );
    let notif = failure_notification_from_event(&row, "https://app.nvbes.test").expect("recipient");
    assert_eq!(
        notif.invoice_url.as_deref(),
        Some("https://stripe.test/in_fail")
    );
}

#[test]
fn failure_notification_builds_portal_url_from_app_url() {
    let row = sample_row(
        "billing.invoice.payment_failed.v1",
        json!({
            "recipient_email": "carol@example.test",
            "invoice_id": "in_fail"
        }),
    );
    let notif = failure_notification_from_event(&row, "https://app.nvbes.test").expect("recipient");
    assert_eq!(
        notif.billing_portal_url.as_deref(),
        Some("https://app.nvbes.test/billing/portal")
    );
    assert_eq!(notif.invoice_id, "in_fail");
}

#[tokio::test]
async fn dispatch_acknowledges_custom_event_without_email_client() {
    if !postgres_reachable() {
        return;
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:15432/nvbes_coverage_test".into()
        }))
        .await
        .expect("postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };
    let state = BillingWorkerState::new(config, pool.clone())
        .await
        .expect("state");
    let event_uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO billing_outbox (event_uuid, event_type, aggregate_id, payload) VALUES ($1, 'billing.custom.noop.v1', $2, '{}'::jsonb)",
    )
    .bind(event_uuid)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("insert");
    let outcome = dispatch_message(&state, &event_uuid.to_string())
        .await
        .expect("dispatch");
    assert_eq!(outcome, DispatchOutcome::Acknowledged);
}

fn postgres_reachable() -> bool {
    crate::test_support::postgres_reachable()
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn dispatch_by_numeric_id_and_missing_event(pool: sqlx::PgPool) {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };
    let state = BillingWorkerState::new(config, pool.clone())
        .await
        .expect("state");

    let event_uuid = Uuid::new_v4();
    let inserted: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO billing_outbox (event_uuid, event_type, aggregate_id, payload)
        VALUES ($1, 'billing.invoice.paid.v1', $2, $3)
        RETURNING id
        "#,
    )
    .bind(event_uuid)
    .bind(Uuid::new_v4())
    .bind(json!({ "invoice_id": "in_by_id" }))
    .fetch_one(&pool)
    .await
    .expect("insert");

    let outcome = dispatch_message(&state, &inserted.0.to_string())
        .await
        .expect("dispatch by id");
    assert_eq!(outcome, DispatchOutcome::Acknowledged);

    let missing = dispatch_message(&state, &Uuid::new_v4().to_string())
        .await
        .expect("missing event acks");
    assert_eq!(missing, DispatchOutcome::Acknowledged);

    let again = dispatch_message(&state, &event_uuid.to_string())
        .await
        .expect("already published acks");
    assert_eq!(again, DispatchOutcome::Acknowledged);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn run_local_dispatches_enqueued_message_then_shuts_down(pool: sqlx::PgPool) {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };
    let state = BillingWorkerState::new(config, pool.clone())
        .await
        .expect("state");
    let event_uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO billing_outbox (event_uuid, event_type, aggregate_id, payload) VALUES ($1, 'billing.custom.noop.v1', $2, '{}'::jsonb)",
    )
    .bind(event_uuid)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("insert");

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(1);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let worker = tokio::spawn(run_local(state, rx, shutdown_rx));
    tx.send(event_uuid.to_string()).await.expect("enqueue");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    shutdown_tx.send(true).expect("shutdown");
    worker.await.expect("worker join");

    let published: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT published_at FROM billing_outbox WHERE event_uuid = $1")
            .bind(event_uuid)
            .fetch_one(&pool)
            .await
            .expect("query");
    assert!(published.is_some());
}
