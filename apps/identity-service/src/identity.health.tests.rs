use axum::{body::Body, http::Request};
use tower::ServiceExt;

use super::router;

#[tokio::test]
async fn probes_are_shallow_and_available_without_a_database() {
    for path in ["/health/live", "/health/ready"] {
        let response = router()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(response.status().is_success());
    }
}

#[tokio::test]
async fn no_public_identity_route_is_exposed() {
    let response = router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}
