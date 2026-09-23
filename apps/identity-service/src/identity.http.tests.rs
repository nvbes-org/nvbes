use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::health::tests::state;

#[tokio::test]
async fn register_is_forbidden_when_public_signup_is_closed() {
    let mut state = state();
    let mut config = (*state.config).clone();
    config.public_signup_enabled = false;
    state.config = std::sync::Arc::new(config);

    let response = crate::http::router(&state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"user@example.com","password":"password-123456"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn login_route_is_present() {
    let response = crate::http::router(&state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}

#[tokio::test]
async fn logout_route_is_present() {
    let response = crate::http::router(&state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
}
