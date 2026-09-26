use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::parse_message_id;
use crate::test_support;

#[test]
fn queue_payload_is_exactly_an_optional_whitespace_wrapped_uuid() {
    let id = uuid::Uuid::new_v4();
    assert_eq!(parse_message_id(format!("  {id}\n").as_bytes()), Some(id));
    assert!(parse_message_id(b"not-a-message-id").is_none());
    assert!(parse_message_id(&[0xff]).is_none());
}

#[tokio::test]
async fn queue_trigger_rejects_invalid_message_identifiers() {
    let state = test_support::state(
        sqlx::postgres::PgPoolOptions::new()
            .min_connections(0)
            .max_connections(1)
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
    );
    let response = super::router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/queue/email-dispatch")
                .body(Body::from("not-a-uuid"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn queue_trigger_reports_unavailable_when_database_is_closed() {
    let state = test_support::state(
        sqlx::postgres::PgPoolOptions::new()
            .min_connections(0)
            .max_connections(1)
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
    );
    state.db.close().await;
    let response = super::router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/queue/email-dispatch")
                .body(Body::from(uuid::Uuid::new_v4().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn queue_trigger_acknowledges_missing_and_retries_busy_messages(pool: sqlx::PgPool) {
    let app = super::router(test_support::state(pool.clone()));
    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/queue/email-dispatch")
                .body(Body::from(uuid::Uuid::new_v4().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NO_CONTENT);

    let command = test_support::command("queue-trigger-busy", "queue-busy@example.com");
    let receipt = crate::database::accept_command(
        &pool,
        &crate::crypto::EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    let message_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(receipt.receipt.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(
        "UPDATE email_messages SET next_attempt_at = clock_timestamp() + INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(message_id)
    .execute(&pool)
    .await
    .unwrap();

    let busy = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/queue/email-dispatch")
                .body(Body::from(message_id.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(busy.status(), StatusCode::SERVICE_UNAVAILABLE);
}
