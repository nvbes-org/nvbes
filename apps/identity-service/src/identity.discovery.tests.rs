use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::discovery::router;
use crate::health::tests::state;

#[tokio::test]
async fn oauth_metadata_endpoint_returns_expected_metadata() {
    let app = router(&state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-authorization-server")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn oidc_metadata_endpoint_returns_expected_configuration() {
    let app = router(&state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/openid-configuration")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn jwks_endpoint_fails_cleanly_without_configured_token_keys() {
    let app = router(&state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}
