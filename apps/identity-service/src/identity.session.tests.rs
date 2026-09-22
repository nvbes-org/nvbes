use axum::http::HeaderMap;

use super::{
    SESSION_COOKIE_NAME, authorize_return_to, session_token_from_headers, validate_return_to,
};

#[test]
fn session_token_prefers_bearer_then_cookie() {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::COOKIE,
        format!("{SESSION_COOKIE_NAME}=cookie-token; other=1")
            .parse()
            .unwrap(),
    );
    assert_eq!(
        session_token_from_headers(&headers).as_deref(),
        Some("cookie-token")
    );

    headers.insert(
        axum::http::header::AUTHORIZATION,
        "Bearer bearer-token".parse().unwrap(),
    );
    assert_eq!(
        session_token_from_headers(&headers).as_deref(),
        Some("bearer-token")
    );
}

#[test]
fn return_to_must_target_authorize_under_issuer() {
    assert_eq!(
        validate_return_to("/oauth/authorize?client_id=a", "https://id.example"),
        Some("/oauth/authorize?client_id=a".into())
    );
    assert_eq!(
        validate_return_to(
            "https://id.example/oauth/authorize?client_id=a",
            "https://id.example"
        ),
        Some("https://id.example/oauth/authorize?client_id=a".into())
    );
    assert!(validate_return_to("https://evil.example/oauth/authorize", "https://id.example").is_none());
    assert!(validate_return_to("/api/v1/auth/login", "https://id.example").is_none());
}

#[test]
fn authorize_return_to_keeps_query() {
    let uri: axum::http::Uri = "/oauth/authorize?response_type=code&client_id=app"
        .parse()
        .unwrap();
    assert_eq!(
        authorize_return_to(&uri),
        "/oauth/authorize?response_type=code&client_id=app"
    );
}
