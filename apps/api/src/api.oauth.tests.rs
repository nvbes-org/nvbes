use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::{app, test_pool};
use crate::config::ApiConfig;

#[tokio::test]
async fn oidc_discovery_is_accessible() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/openid-configuration")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["issuer"], "http://localhost:8080");
    assert_eq!(body["jwks_uri"], "http://localhost:8080/oauth/jwks");
}

#[tokio::test]
async fn oidc_jwks_is_accessible() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/oauth/jwks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["keys"].is_array());
}

/// Without a transport peer Identity refuses throttled routes before any
/// database access, so this holds even with no database reachable.
#[tokio::test]
async fn protected_oauth_routes_refuse_requests_without_a_peer() {
    let response = app(test_pool(), &ApiConfig::from_env().unwrap())
        .oneshot(empty_token_request())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// With the peer the request reaches protocol validation: the empty form is
/// rejected as a client error instead of an unavailable service.
#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "../../libs/rust/identity/migrations")]
async fn protected_oauth_routes_reach_validation_with_a_peer(pool: sqlx::PgPool) {
    let mut request = empty_token_request();
    request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [203, 0, 113, 10],
            40_000,
        ))));
    let response = app(pool, &ApiConfig::from_env().unwrap())
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_ne!(status, StatusCode::SERVICE_UNAVAILABLE, "{body:?}");
    assert!(status.is_client_error(), "{status} {body:?}");
}

fn empty_token_request() -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn oauth_routes_are_mounted_on_api() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    // /oauth/authorize should not return 404
    let auth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(auth.status(), StatusCode::NOT_FOUND);

    // /oauth/token should not return 404
    let token = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(token.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn password_recovery_routes_are_mounted() {
    let response = app(test_pool(), &ApiConfig::from_env().unwrap())
        .oneshot(
            Request::builder()
                .uri("/oauth/password-recovery/context")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}
