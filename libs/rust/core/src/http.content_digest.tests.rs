use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::from_fn,
    routing::post,
};
use tower::ServiceExt;

use super::{content_digest_guard, content_digest_header_value, parse_sha256_digest};

fn digest_app() -> Router {
    Router::new()
        .route("/echo", post(|body: Body| async move { body }))
        .layer(from_fn(content_digest_guard))
}

#[tokio::test]
async fn guard_passes_through_when_content_digest_header_is_absent() {
    let response = digest_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .body(Body::from("hello"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn guard_accepts_matching_sha256_digest() {
    let body = b"payload-bytes";
    let digest = content_digest_header_value(body);
    let response = digest_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .header("content-digest", digest)
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn guard_rejects_mismatched_digest() {
    let response = digest_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .header("content-digest", content_digest_header_value(b"expected"))
                .body(Body::from("actual"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_ignores_non_sha256_digest_algorithms() {
    let response = digest_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .header("content-digest", "sha-512=:abcd:")
                .body(Body::from("hello"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn guard_rejects_non_utf8_content_digest_header() {
    let mut request = Request::builder()
        .method("POST")
        .uri("/echo")
        .body(Body::from("hello"))
        .unwrap();
    request.headers_mut().insert(
        "content-digest",
        HeaderValue::from_bytes(&[0xff, 0xfe]).expect("binary header"),
    );
    let response = digest_app().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn parse_sha256_digest_accepts_colon_wrapped_and_bare_forms() {
    assert_eq!(
        parse_sha256_digest("sha-256=:abc123:"),
        Some("abc123".to_string())
    );
    assert_eq!(
        parse_sha256_digest("sha-256=abc123"),
        Some("abc123".to_string())
    );
    assert_eq!(parse_sha256_digest("sha-256="), None);
    assert_eq!(parse_sha256_digest("sha-256=:ab cd:"), None);
}
