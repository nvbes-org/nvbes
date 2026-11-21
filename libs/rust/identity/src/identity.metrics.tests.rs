use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::health::tests::state;

use super::router;

#[tokio::test]
async fn metrics_require_the_exact_bearer_token() {
    let app = router(state());
    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), axum::http::StatusCode::UNAUTHORIZED);

    let authorized = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header(
                    "authorization",
                    "Bearer identity-metrics-test-token-32-characters",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), axum::http::StatusCode::OK);
}
