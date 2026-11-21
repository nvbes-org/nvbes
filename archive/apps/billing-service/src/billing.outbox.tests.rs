use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use nvbes_email::{
    EmailClient, EmailClientConfig,
    proto::nvbes::email::v1::{
        EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
        email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
    },
};
use serde_json::json;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;

use super::{publish_pending_outbox_events, record_outbox_event};

#[derive(Clone, Default)]
struct CapturingEmailService {
    requests: Arc<Mutex<Vec<String>>>,
}

#[tonic::async_trait]
impl EmailDeliveryService for CapturingEmailService {
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        self.requests
            .lock()
            .expect("lock")
            .push(request.get_ref().producer.clone());
        let accepted_at = Utc::now();
        Ok(Response::new(ProtoEmailReceipt {
            message_id: format!("msg_{}", Uuid::new_v4().simple()),
            accepted_at: Some(prost_types::Timestamp {
                seconds: accepted_at.timestamp(),
                nanos: accepted_at.timestamp_subsec_nanos() as i32,
            }),
            deliver_before: None,
            duplicate: false,
        }))
    }
}

async fn connected_email_client() -> (
    EmailClient,
    Arc<Mutex<Vec<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind email mock");
    let address = listener.local_addr().expect("addr");
    let service = CapturingEmailService::default();
    let requests = service.requests.clone();
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
            "billing-outbox-test-token-at-least-32-chars".into(),
            Duration::from_secs(2),
        )
        .expect("email config"),
    )
    .await
    .expect("email connect");
    (client, requests, handle)
}

#[sqlx::test(migrations = "./migrations")]
async fn records_and_publishes_pending_outbox(pool: PgPool) {
    let account_id = Uuid::new_v4();
    record_outbox_event(
        &pool,
        "billing.subscription.changed.v1",
        account_id,
        &json!({ "status": "active" }),
    )
    .await
    .expect("record");

    let published = publish_pending_outbox_events(&pool, None, "https://nvbes.test", 10)
        .await
        .expect("publish");
    assert_eq!(published, 1);

    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE published_at IS NULL")
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(pending, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn publishes_invoice_notification_event_types(pool: PgPool) {
    let account_id = Uuid::new_v4();
    for (event_type, payload) in [
        (
            "billing.invoice.paid.v1",
            json!({
                "recipient_email": "paid@t.test",
                "amount_minor": 1200,
                "currency": "EUR",
                "invoice_id": "in_paid"
            }),
        ),
        (
            "billing.invoice.payment_failed.v1",
            json!({
                "recipient_email": "failed@t.test",
                "amount_minor": 900,
                "currency": "EUR",
                "invoice_id": "in_failed"
            }),
        ),
        ("billing.custom.event.v1", json!({ "ignored": true })),
    ] {
        record_outbox_event(&pool, event_type, account_id, &payload)
            .await
            .expect("record");
    }

    let published = publish_pending_outbox_events(&pool, None, "https://nvbes.test", 10)
        .await
        .expect("publish");
    assert_eq!(published, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn publishes_invoice_events_through_email_client(pool: PgPool) {
    let (client, requests, server) = connected_email_client().await;
    let account_id = Uuid::new_v4();

    for (event_type, payload) in [
        (
            "billing.invoice.paid.v1",
            json!({
                "recipient_email": "paid@t.test",
                "customer_name": "Ada",
                "amount_minor": 1200,
                "currency": "EUR",
                "invoice_id": "in_paid",
                "invoice_url": "https://invoice.test/paid"
            }),
        ),
        (
            "billing.invoice.paid.v1",
            json!({ "amount_minor": 1, "invoice_id": "in_no_email" }),
        ),
        (
            "billing.invoice.payment_failed.v1",
            json!({
                "recipient_email": "failed@t.test",
                "amount_minor": 900,
                "currency": "USD",
                "invoice_id": "in_failed",
                "invoice_url": "https://invoice.test/failed"
            }),
        ),
        (
            "billing.invoice.payment_failed.v1",
            json!({ "invoice_id": "in_failed_no_email" }),
        ),
        ("billing.custom.event.v1", json!({ "ignored": true })),
    ] {
        record_outbox_event(&pool, event_type, account_id, &payload)
            .await
            .expect("record");
    }

    let published = publish_pending_outbox_events(&pool, Some(&client), "https://nvbes.test", 10)
        .await
        .expect("publish with email");
    assert_eq!(published, 5);

    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE published_at IS NULL")
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(pending, 0);
    let captured = requests.lock().expect("lock").clone();
    assert_eq!(
        captured.len(),
        2,
        "paid and payment_failed arms must each deliver when recipient_email is present"
    );
    assert!(
        captured
            .iter()
            .all(|producer| producer == "billing-service")
    );
    server.abort();
}
