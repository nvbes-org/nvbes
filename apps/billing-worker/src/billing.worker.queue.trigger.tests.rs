use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{config::BillingWorkerConfig, state::BillingWorkerState};

async fn test_state() -> BillingWorkerState {
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

    // Lazy pool: safe for branches that never open a connection.
    BillingWorkerState::new(config, db).await.unwrap()
}

#[tokio::test]
async fn trigger_dispatch_handles_empty_payload() {
    let app = super::router(test_state().await);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/internal/queue/billing-dispatch")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn trigger_dispatch_rejects_invalid_utf8_payload() {
    let app = super::router(test_state().await);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/internal/queue/billing-dispatch")
                .method("POST")
                .body(Body::from(vec![0xff, 0xfe, 0xfd]))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn trigger_dispatch_treats_whitespace_as_empty() {
    let app = super::router(test_state().await);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/internal/queue/billing-dispatch")
                .method("POST")
                .body(Body::from("   \n\t  "))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn reconcile_returns_service_unavailable_when_database_is_unreachable() {
    let state = test_state().await;
    state.db.close().await;
    let app = super::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/internal/reconciliation")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn dispatch_returns_service_unavailable_when_database_is_unreachable() {
    let state = test_state().await;
    state.db.close().await;
    let app = super::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/internal/queue/billing-dispatch")
                .method("POST")
                .body(Body::from(uuid::Uuid::new_v4().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
