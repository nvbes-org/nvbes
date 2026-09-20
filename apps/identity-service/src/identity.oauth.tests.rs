use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::health::tests::state;
use crate::oauth::router;
use crate::oauth::types::{validate_client_id, validate_redirect_uri, validate_scope};

#[test]
fn validate_client_id_accepts_valid_formats() {
    assert!(validate_client_id("my-app"));
    assert!(validate_client_id("my_app_123"));
    assert!(validate_client_id("test-client-v1"));
    assert!(validate_client_id("a"));
    assert!(validate_client_id("client_with_underscores_and-hyphens"));
}

#[test]
fn validate_client_id_rejects_invalid_formats() {
    assert!(!validate_client_id(""));
    assert!(!validate_client_id("client with spaces"));
    assert!(!validate_client_id("client@domain"));
    assert!(!validate_client_id("client!exclamation"));
    assert!(!validate_client_id("client.dots"));
}

#[test]
fn validate_redirect_uri_accepts_valid_urls() {
    assert!(validate_redirect_uri("https://example.com/callback"));
    assert!(validate_redirect_uri("http://localhost:3000/callback"));
    assert!(validate_redirect_uri(
        "https://app.example.com/auth/callback"
    ));
}

#[test]
fn validate_redirect_uri_rejects_invalid_urls() {
    assert!(!validate_redirect_uri(""));
    assert!(!validate_redirect_uri("not-a-url"));
    assert!(!validate_redirect_uri("ftp://example.com/callback"));
}

#[test]
fn validate_scope_accepts_valid_scopes() {
    assert!(validate_scope("openid"));
    assert!(validate_scope("openid profile email"));
    assert!(validate_scope("account:read account:write"));
    assert!(validate_scope(""));
}

#[test]
fn validate_scope_rejects_invalid_scopes() {
    assert!(!validate_scope("scope@invalid"));
    assert!(!validate_scope("scope!exclamation"));
}

#[tokio::test]
async fn authorize_rejects_invalid_response_type() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=token&client_id=client&redirect_uri=https://example.com/cb")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn authorize_rejects_invalid_client_id() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=invalid%20id&redirect_uri=https://example.com/cb")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn authorize_rejects_invalid_redirect_uri() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=ftp://not-allowed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn authorize_rejects_invalid_scope() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=https://example.com/cb&scope=invalid@scope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn token_rejects_unsupported_grant_type() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"grant_type":"password"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn token_rejects_missing_code_in_auth_code_grant() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"grant_type":"authorization_code"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn token_rejects_missing_refresh_token_in_refresh_grant() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"grant_type":"refresh_token"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}
