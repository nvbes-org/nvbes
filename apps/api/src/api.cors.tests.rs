use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

use super::{app, test_pool};
use crate::config::ApiConfig;

async fn preflight(path: &str, origin: &str) -> axum::http::Response<Body> {
    app(test_pool(), &ApiConfig::from_env().unwrap())
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri(path)
                .header(header::ORIGIN, origin)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn unknown_origins_get_no_cors_grant_anywhere() {
    for path in ["/oauth/token", "/api/v1/profile", "/healthz"] {
        let response = preflight(path, "https://evil.test").await;
        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            None,
            "{path}"
        );
    }
}

#[tokio::test]
async fn registered_client_origin_gets_its_exact_origin_never_a_wildcard() {
    let response = preflight("/oauth/token", "http://localhost:3001").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("http://localhost:3001")
    );
}
