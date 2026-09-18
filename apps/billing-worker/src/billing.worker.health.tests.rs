use axum::{body::Body, http::Request};
use tower::ServiceExt;

use super::live;

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

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
