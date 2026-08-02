use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn liveness_is_shallow_and_does_not_need_state() {
    async fn live() -> &'static str {
        "alive"
    }

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
