use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{config::BillingWorkerConfig, state::BillingWorkerState};

use super::{live, router};

async fn unreachable_state() -> BillingWorkerState {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://127.0.0.1:1/unreachable".into(),
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
        .connect_lazy("postgres://127.0.0.1:1/unreachable")
        .unwrap();

    BillingWorkerState::new(config, db).await.unwrap()
}

#[tokio::test]
async fn liveness_returns_alive_payload() {
    let payload = live().await;
    assert_eq!(payload.0.status, "alive");
    assert_eq!(payload.0.service, "nvbes-billing-worker");

    let response = axum::Router::new()
        .route("/health/live", axum::routing::get(live))
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
async fn readiness_reports_not_ready_when_database_is_unreachable() {
    let app = router(unreachable_state().await);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
