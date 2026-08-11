use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

use super::{SnsMessage, apply_notification, confirm_subscription};
use crate::test_support;

fn message(payload: &str) -> SnsMessage {
    SnsMessage {
        message_type: "Notification".to_string(),
        message_id: format!("sns-{}", uuid::Uuid::new_v4()),
        topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
        message: payload.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        signature_version: "1".to_string(),
        signature: "invalid".to_string(),
        signing_cert_url: "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem".to_string(),
        subject: None,
        token: None,
        subscribe_url: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn webhook_router_rejects_bad_content_types_and_disabled_verification(pool: sqlx::PgPool) {
    let router = super::router(test_support::state(pool));
    let response = router
        .clone()
        .oneshot(
            Request::post("/webhooks/scaleway/topics-and-events")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let response = router
        .oneshot(
            Request::post("/webhooks/scaleway/topics-and-events")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test(migrations = "./migrations")]
async fn notification_processing_handles_valid_incomplete_and_duplicate_payloads(
    pool: sqlx::PgPool,
) {
    let state = test_support::state(pool);
    let incomplete = message(r#"{"id":"","email_id":"","type":""}"#);
    assert!(apply_notification(&state, &incomplete).await.is_err());

    let valid = message(
        r#"{"id":"provider-event-1","email_id":"unknown-provider-id","type":"future_event","status":"stored"}"#,
    );
    let first = apply_notification(&state, &valid).await.unwrap();
    assert_eq!(first, ("processed", "stored_unknown"));
    let duplicate = apply_notification(&state, &valid).await.unwrap();
    assert_eq!(duplicate, ("duplicate", "duplicate"));

    let confirmation = SnsMessage {
        message_type: "SubscriptionConfirmation".to_string(),
        ..message("confirmation")
    };
    assert!(confirm_subscription(&state, &confirmation).await.is_err());
}
