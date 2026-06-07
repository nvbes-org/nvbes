use super::auth::token_client_auth;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue};
use base64::Engine;

fn basic_headers(client_id: &str, client_secret: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
    );
    headers
}

#[test]
fn token_client_auth_rejects_inconsistent_body_credentials() {
    let headers = basic_headers("client-a", "secret-a");

    let error = token_client_auth(&headers, Some("client-b"), Some("secret-a"), None, None)
        .expect_err("mismatched client_id should fail");

    assert_eq!(error.code, "invalid_client");
}

#[test]
fn token_client_auth_requires_body_client_id_without_basic_auth() {
    let headers = HeaderMap::new();

    let error = token_client_auth(&headers, None, Some("secret-a"), None, None)
        .expect_err("missing client_id should fail");

    assert_eq!(error.code, "missing_client_id");
}

#[test]
fn token_client_auth_accepts_body_credentials_without_basic_auth() {
    let headers = HeaderMap::new();

    let auth = token_client_auth(&headers, Some("client-a"), Some("secret-a"), None, None)
        .expect("body credentials should be accepted");

    assert_eq!(auth.client_id, "client-a");
    assert_eq!(auth.client_secret.as_deref(), Some("secret-a"));
}

#[test]
fn token_client_auth_rejects_multiple_auth_methods() {
    let headers = basic_headers("client-a", "secret-a");

    let error = token_client_auth(
        &headers,
        Some("client-a"),
        None,
        Some(crate::domains::oauth::client_assertion::CLIENT_ASSERTION_TYPE_JWT_BEARER),
        Some("not.a.jwt"),
    )
    .expect_err("mixed auth methods should fail");

    assert_eq!(error.code, "invalid_client");
}
