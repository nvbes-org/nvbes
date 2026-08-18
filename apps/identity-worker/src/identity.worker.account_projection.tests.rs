use std::time::Duration;

use nvbes_observability::metrics::HttpMetrics;
use serde_json::json;
use sqlx::{PgPool, Row};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use super::{client::AccountProjectionClient, process_next_with, refresh_metrics_with};
use crate::worker::test_support::database_pool;

const EVENT_TYPE: &str = "identity.principal.registered.v1";
const AGGREGATE_TYPE: &str = "identity-worker-projection-orchestration-test";

#[tokio::test]
async fn process_next_returns_false_when_the_outbox_is_empty() {
    let db = database_pool().await;
    cleanup(&db).await;
    let client = client(unused_endpoint().await);

    assert!(!process_next_with(&db, &client).await.expect("empty queue"));
}

#[tokio::test]
async fn process_next_deletes_successful_deliveries_end_to_end() {
    let db = database_pool().await;
    cleanup(&db).await;
    let event_id = insert_event(&db).await;
    let (endpoint, server) = response_server("204 No Content").await;

    assert!(
        process_next_with(&db, &client(endpoint))
            .await
            .expect("process")
    );
    server.await.expect("HTTP server");

    assert!(!event_exists(&db, event_id).await);
}

#[tokio::test]
async fn process_next_dead_letters_permanent_delivery_failures_end_to_end() {
    let db = database_pool().await;
    cleanup(&db).await;
    let event_id = insert_event(&db).await;
    let (endpoint, server) = response_server("400 Bad Request").await;

    assert!(
        process_next_with(&db, &client(endpoint))
            .await
            .expect("process")
    );
    server.await.expect("HTTP server");

    let row = sqlx::query(
        "SELECT dead_lettered_at, last_error_code FROM identity_outbox_events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_one(&db)
    .await
    .expect("failed event");
    assert!(
        row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("dead_lettered_at")
            .is_some()
    );
    assert_eq!(
        row.get::<String, _>("last_error_code"),
        "account_projection_rejected"
    );
    cleanup(&db).await;
}

#[tokio::test]
async fn refresh_metrics_exports_pending_and_dead_letter_series() {
    let db = database_pool().await;
    cleanup(&db).await;
    let pending = insert_event(&db).await;
    let dead = insert_event(&db).await;
    sqlx::query("UPDATE identity_outbox_events SET dead_lettered_at = NOW() WHERE id = $1")
        .bind(dead)
        .execute(&db)
        .await
        .expect("dead letter fixture");
    let metrics = HttpMetrics::default();

    refresh_metrics_with(&db, &metrics)
        .await
        .expect("refresh metrics");

    let rendered = metrics.render();
    assert!(rendered.contains("account_registration_projection"));
    assert!(rendered.contains("status=\"pending\""));
    assert!(rendered.contains("status=\"dead_letter\""));
    assert!(event_exists(&db, pending).await);
    cleanup(&db).await;
}

fn client(mut endpoint: reqwest::Url) -> AccountProjectionClient {
    endpoint.set_path("/internal/v1/identity-registrations");
    AccountProjectionClient::new(endpoint, "projection-test-token-0000000001")
        .expect("projection client")
}

async fn response_server(status: &'static str) -> (reqwest::Url, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let endpoint = reqwest::Url::parse(&format!("http://{}", listener.local_addr().expect("addr")))
        .expect("URL");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut request = [0; 4096];
        let read = stream.read(&mut request).await.expect("read");
        assert!(
            String::from_utf8_lossy(&request[..read])
                .contains("POST /internal/v1/identity-registrations")
        );
        stream
            .write_all(
                format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .expect("respond");
    });
    (endpoint, server)
}

async fn unused_endpoint() -> reqwest::Url {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let endpoint = reqwest::Url::parse(&format!("http://{}", listener.local_addr().expect("addr")))
        .expect("URL");
    drop(listener);
    endpoint
}

async fn insert_event(db: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO identity_outbox_events (id, aggregate_type, aggregate_id, event_type, payload) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(AGGREGATE_TYPE)
    .bind(Uuid::new_v4())
    .bind(EVENT_TYPE)
    .bind(json!({"event_id": id}))
    .execute(db)
    .await
    .expect("insert event");
    id
}

async fn event_exists(db: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_outbox_events WHERE id = $1)")
        .bind(id)
        .fetch_one(db)
        .await
        .expect("event existence")
}

async fn cleanup(db: &PgPool) {
    sqlx::query("DELETE FROM identity_outbox_events WHERE aggregate_type = $1")
        .bind(AGGREGATE_TYPE)
        .execute(db)
        .await
        .expect("cleanup");
    tokio::time::sleep(Duration::from_millis(1)).await;
}
