#[cfg(feature = "database-tests")]
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "database-tests")]
use chrono::Utc;
#[cfg(feature = "database-tests")]
use nvbes_email::{
    EmailClient, EmailClientConfig,
    proto::nvbes::email::v1::{
        EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
        email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
    },
};
#[cfg(feature = "database-tests")]
use serde_json::json;
#[cfg(feature = "database-tests")]
use tokio::net::TcpListener;
use tokio::sync::watch;
#[cfg(feature = "database-tests")]
use tokio_stream::wrappers::TcpListenerStream;
#[cfg(feature = "database-tests")]
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;

use crate::{config::BillingWorkerConfig, state::BillingWorkerState};

#[cfg(not(feature = "database-tests"))]
use super::run_local;
#[cfg(feature = "database-tests")]
use super::{DispatchOutcome, dispatch_message, run_local};

#[cfg(feature = "database-tests")]
#[derive(Clone)]
struct MockEmailService {
    fail: Arc<Mutex<bool>>,
}

#[cfg(feature = "database-tests")]
#[tonic::async_trait]
impl EmailDeliveryService for MockEmailService {
    async fn submit_email(
        &self,
        _request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        if *self.fail.lock().expect("lock") {
            return Err(Status::unavailable("email mock forced failure"));
        }
        let accepted_at = Utc::now();
        let deliver_before = accepted_at + chrono::Duration::hours(24);
        Ok(Response::new(ProtoEmailReceipt {
            message_id: format!("msg_{}", Uuid::new_v4().simple()),
            accepted_at: Some(prost_types::Timestamp {
                seconds: accepted_at.timestamp(),
                nanos: accepted_at.timestamp_subsec_nanos() as i32,
            }),
            deliver_before: Some(prost_types::Timestamp {
                seconds: deliver_before.timestamp(),
                nanos: deliver_before.timestamp_subsec_nanos() as i32,
            }),
            duplicate: false,
        }))
    }
}

#[cfg(feature = "database-tests")]
async fn connected_email_client(fail: bool) -> (EmailClient, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind email mock");
    let address = listener.local_addr().expect("addr");
    let service = MockEmailService {
        fail: Arc::new(Mutex::new(fail)),
    };
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(EmailDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("serve email mock");
    });
    let client = EmailClient::connect(
        EmailClientConfig::from_values(
            "development",
            format!("http://{address}"),
            "billing-worker-email-token-at-least-32-chars".into(),
            Duration::from_secs(2),
        )
        .expect("email config"),
    )
    .await
    .expect("email connect");
    (client, handle)
}

fn base_config() -> BillingWorkerConfig {
    BillingWorkerConfig {
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
    }
}

#[cfg(feature = "database-tests")]
async fn state_with_email(client: EmailClient, pool: sqlx::PgPool) -> BillingWorkerState {
    let mut state = BillingWorkerState::new(base_config(), pool)
        .await
        .expect("state");
    state.email_client = Some(Arc::new(client));
    state
}

#[cfg(feature = "database-tests")]
async fn insert_outbox(pool: &sqlx::PgPool, event_type: &str, payload: serde_json::Value) -> Uuid {
    let event_uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO billing_outbox (event_uuid, event_type, aggregate_id, payload) VALUES ($1, $2, $3, $4)",
    )
    .bind(event_uuid)
    .bind(event_type)
    .bind(Uuid::new_v4())
    .bind(payload)
    .execute(pool)
    .await
    .expect("insert");
    event_uuid
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn process_event_delivers_receipt_and_failure_through_email_client(pool: sqlx::PgPool) {
    let (client, server) = connected_email_client(false).await;
    let state = state_with_email(client, pool.clone()).await;

    let paid = insert_outbox(
        &pool,
        "billing.invoice.paid.v1",
        json!({
            "recipient_email": "paid@nvbes.test",
            "amount_minor": 1200,
            "currency": "EUR",
            "invoice_id": "in_paid_email"
        }),
    )
    .await;
    assert_eq!(
        dispatch_message(&state, &paid.to_string())
            .await
            .expect("paid"),
        DispatchOutcome::Acknowledged
    );

    let failed = insert_outbox(
        &pool,
        "billing.invoice.payment_failed.v1",
        json!({
            "recipient_email": "fail@nvbes.test",
            "invoice_id": "in_fail_email"
        }),
    )
    .await;
    assert_eq!(
        dispatch_message(&state, &failed.to_string())
            .await
            .expect("failed"),
        DispatchOutcome::Acknowledged
    );

    let sparse_paid = insert_outbox(
        &pool,
        "billing.invoice.paid.v1",
        json!({ "invoice_id": "x" }),
    )
    .await;
    assert_eq!(
        dispatch_message(&state, &sparse_paid.to_string())
            .await
            .expect("sparse paid"),
        DispatchOutcome::Acknowledged
    );

    let sparse_fail = insert_outbox(&pool, "billing.invoice.payment_failed.v1", json!({})).await;
    assert_eq!(
        dispatch_message(&state, &sparse_fail.to_string())
            .await
            .expect("sparse fail"),
        DispatchOutcome::Acknowledged
    );

    let custom = insert_outbox(&pool, "billing.custom.noop.v1", json!({})).await;
    assert_eq!(
        dispatch_message(&state, &custom.to_string())
            .await
            .expect("custom"),
        DispatchOutcome::Acknowledged
    );

    server.abort();
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn process_event_email_failure_signals_retry(pool: sqlx::PgPool) {
    let (client, server) = connected_email_client(true).await;
    let state = state_with_email(client, pool.clone()).await;
    let event = insert_outbox(
        &pool,
        "billing.invoice.paid.v1",
        json!({
            "recipient_email": "retry@nvbes.test",
            "invoice_id": "in_retry"
        }),
    )
    .await;
    assert_eq!(
        dispatch_message(&state, &event.to_string())
            .await
            .expect("retry"),
        DispatchOutcome::Retry
    );
    server.abort();
}

#[tokio::test]
async fn run_local_logs_dispatch_errors_when_database_is_closed() {
    let config = base_config();
    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let state = BillingWorkerState::new(config, db).await.unwrap();
    state.db.close().await;

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(1);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let worker = tokio::spawn(run_local(state, rx, shutdown_rx));
    tx.send(Uuid::new_v4().to_string()).await.expect("enqueue");
    tokio::time::sleep(Duration::from_millis(100)).await;
    shutdown_tx.send(true).expect("shutdown");
    worker.await.expect("join");
}
