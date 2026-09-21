use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::authz::router;
use crate::health::tests::state;

#[tokio::test]
async fn introspect_fails_cleanly_without_configured_token_keys() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/introspect")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"token":"invalid.token.here"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn authz_decision_fails_cleanly_without_configured_token_keys() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/authz/decision")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"token":"invalid.token","resource":"user","action":"read"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}
