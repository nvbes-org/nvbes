use super::*;
use axum::{
    body::Body,
    http::{header, Method, Request},
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::cockpit_actions::OperatorActionPayload;

fn setup_test_app() -> Router {
    let state = PlatformCockpitState {
        environment: "staging".to_string(),
        auth_policy: Arc::new(OperatorAuthPolicy::new("secret-operator-token")),
        health_aggregator: Arc::new(HealthAggregator::default()),
        finops_monitor: Arc::new(FinOpsMonitor::default()),
    };
    create_platform_cockpit_router(state)
}

#[tokio::test]
async fn health_probes_are_public_and_healthy() {
    let app = setup_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_unauthenticated_cockpit_access() {
    let app = setup_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/overview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn allows_authenticated_operator_overview() {
    let app = setup_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/overview")
                .header(header::AUTHORIZATION, "Bearer secret-operator-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn dispatches_authenticated_operator_action() {
    let app = setup_test_app();
    let cmd = OperatorCommand {
        operator_id: "solo_operator".to_string(),
        reason: "Valid test mitigation reason".to_string(),
        idempotency_key: "test-key-01".to_string(),
        correlation_id: Uuid::new_v4(),
        payload: OperatorActionPayload::ApplyEmailSuppression {
            email: "bad@domain.com".to_string(),
            reason: "Spam complaints".to_string(),
        },
    };

    let body = serde_json::to_vec(&cmd).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/actions")
                .header(header::AUTHORIZATION, "Bearer secret-operator-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_action_with_invalid_reason() {
    let app = setup_test_app();
    let cmd = OperatorCommand {
        operator_id: "solo_operator".to_string(),
        reason: "x".to_string(), // too short (< 3)
        idempotency_key: "test-key-02".to_string(),
        correlation_id: Uuid::new_v4(),
        payload: OperatorActionPayload::ReleaseEmailSuppression {
            email: "ok@domain.com".to_string(),
        },
    };

    let body = serde_json::to_vec(&cmd).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/actions")
                .header(header::AUTHORIZATION, "Bearer secret-operator-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
