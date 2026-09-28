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
async fn authorize_requires_authentication_with_return_to() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=https://example.com/cb")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "authentication_required");
    assert!(
        json["return_to"]
            .as_str()
            .unwrap()
            .starts_with("/oauth/authorize?")
    );
}

#[tokio::test]
async fn token_rejects_unsupported_grant_type() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("grant_type=password"))
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
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("grant_type=authorization_code"))
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
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("grant_type=refresh_token"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn authorize_rejects_invalid_pkce_method_and_challenge_length() {
    let app = router(&state());
    let invalid_method = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=https://example.com/cb&code_challenge=abc&code_challenge_method=S512")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_method.status(), axum::http::StatusCode::BAD_REQUEST);

    let empty_challenge = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=https://example.com/cb&code_challenge=&code_challenge_method=S256")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        empty_challenge.status(),
        axum::http::StatusCode::BAD_REQUEST
    );

    let oversized = format!("a{}", "b".repeat(128));
    let long_challenge = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=https://example.com/cb&code_challenge={oversized}&code_challenge_method=plain"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(long_challenge.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn authorize_requires_session_before_issuing_code() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=valid-client&redirect_uri=https://example.com/cb")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Without a live DB this fails while resolving redirect allow-list, or
    // returns unauthorized if the client check is skipped. Either way it must
    // not succeed.
    assert!(!res.status().is_success());
}

#[tokio::test]
async fn token_rejects_invalid_scope_on_token_endpoint() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "grant_type=authorization_code&scope=bad%40scope&code=x&redirect_uri=https%3A%2F%2Fexample.com%2Fcb&client_id=c&client_secret=s",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn token_rejects_missing_client_id_and_redirect_uri() {
    let app = router(&state());
    let missing_client = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "grant_type=authorization_code&code=abc&redirect_uri=https%3A%2F%2Fexample.com%2Fcb",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_client.status(), axum::http::StatusCode::BAD_REQUEST);

    let missing_redirect = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "grant_type=authorization_code&code=abc&client_id=valid-client",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        missing_redirect.status(),
        axum::http::StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn token_authorization_code_returns_server_error_without_database() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "grant_type=authorization_code&code=abc&client_id=valid-client&redirect_uri=https%3A%2F%2Fexample.com%2Fcb&client_secret=secret",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.status().is_server_error() || res.status() == axum::http::StatusCode::UNAUTHORIZED,
        "status={}",
        res.status()
    );
}

#[tokio::test]
async fn token_refresh_rejects_unknown_token_without_database() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "grant_type=refresh_token&refresh_token=missing&client_id=valid-client",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.status().is_client_error() || res.status().is_server_error(),
        "status={}",
        res.status()
    );
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn authorize_redirects_with_code_when_session_is_valid(pool: sqlx::PgPool) {
    use crate::database::database_test_support::{
        TokenEnvGuard, identity_state, seed_confidential_oauth_client, seed_principal_and_session,
    };

    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = "db-test-authorize-client";
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_confidential_oauth_client(&state.db, client_id, "unused-secret", redirect_uri).await;
    let (_, _, session_secret) = seed_principal_and_session(&state.db).await;

    let response = router(&state)
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}&scope=account:read&state=xyz"
                ))
                .header("authorization", format!("Bearer {session_secret}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("redirect location");
    assert!(location.starts_with(redirect_uri));
    assert!(location.contains("code="));
    assert!(location.contains("state=xyz"));
}

#[test]
fn validate_helpers_reject_oversized_inputs() {
    assert!(!validate_client_id(&"a".repeat(129)));
    assert!(!validate_redirect_uri(&format!(
        "https://example.com/{}",
        "x".repeat(2040)
    )));
    assert!(!validate_scope(&format!("{}:{}", "a".repeat(65), "b")));
}
