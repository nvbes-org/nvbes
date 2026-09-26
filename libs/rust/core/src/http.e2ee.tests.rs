use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderMap, Request, StatusCode},
    middleware::from_fn_with_state,
    routing::post,
};
use base64::Engine;
use tower::ServiceExt;

use crate::config::AppConfig;

use super::{aad, derive_key, request_e2ee_guard};

const TEST_SECRET: &str = "0123456789abcdef0123456789abcdef";
const TEST_KEY_ID: &str = "kid_test";

fn e2ee_config(required: bool) -> AppConfig {
    AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_required: required,
        request_e2ee_key_id: TEST_KEY_ID.to_string(),
        request_e2ee_secret: Some(TEST_SECRET.to_string()),
        ..AppConfig::default()
    }
}

fn encryption_headers(salt: &[u8], nonce: &[u8], key_id: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-nvbes-e2ee",
        "aes-256-gcm".parse().expect("algorithm header"),
    );
    headers.insert(
        "x-nvbes-e2ee-key-id",
        key_id.parse().expect("key id header"),
    );
    headers.insert(
        "x-nvbes-e2ee-salt",
        base64::engine::general_purpose::STANDARD
            .encode(salt)
            .parse()
            .expect("salt header"),
    );
    headers.insert(
        "x-nvbes-e2ee-nonce",
        base64::engine::general_purpose::STANDARD
            .encode(nonce)
            .parse()
            .expect("nonce header"),
    );
    headers
}

fn encrypt_body(
    secret: &[u8],
    salt: &[u8],
    nonce: &[u8],
    method: &axum::http::Method,
    path: &str,
    plaintext: &[u8],
) -> Vec<u8> {
    let key = derive_key(secret, salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");
    cipher
        .encrypt(
            &Nonce::from(*<&[u8; 12]>::try_from(nonce).expect("nonce length")),
            Payload {
                msg: plaintext,
                aad: &aad(method, path),
            },
        )
        .expect("encrypt")
}

fn e2ee_app(config: AppConfig) -> Router {
    Router::new()
        .route("/auth/login", post(|body: Bytes| async move { body }))
        .layer(from_fn_with_state(config.clone(), request_e2ee_guard))
        .with_state(config)
}

#[tokio::test]
async fn guard_passes_through_when_e2ee_disabled() {
    let config = AppConfig {
        request_e2ee_enabled: false,
        ..AppConfig::default()
    };
    let app = e2ee_app(config);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .body(Body::from(r#"{"plain":true}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn guard_passes_through_without_headers_when_optional() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .body(Body::from("plain-body"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn guard_requires_algorithm_header_when_required() {
    let config = e2ee_config(true);
    let app = e2ee_app(config);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_rejects_unsupported_algorithm() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("x-nvbes-e2ee", "chacha20-poly1305")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_rejects_unknown_key_id() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let salt = b"request-salt-12345";
    let nonce = b"unique nonce";
    let headers = encryption_headers(salt, nonce, "wrong-key");
    let mut builder = Request::builder().method("POST").uri("/auth/login");
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_rejects_short_salt() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let salt = b"short";
    let nonce = b"unique nonce";
    let headers = encryption_headers(salt, nonce, TEST_KEY_ID);
    let mut builder = Request::builder().method("POST").uri("/auth/login");
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_rejects_invalid_nonce_length() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let salt = b"request-salt-12345";
    let nonce = b"too-short";
    let headers = encryption_headers(salt, nonce, TEST_KEY_ID);
    let mut builder = Request::builder().method("POST").uri("/auth/login");
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_returns_internal_error_without_valid_secret() {
    let config = AppConfig {
        request_e2ee_enabled: true,
        request_e2ee_required: false,
        request_e2ee_key_id: TEST_KEY_ID.to_string(),
        request_e2ee_secret: Some("short".to_string()),
        ..AppConfig::default()
    };
    let app = e2ee_app(config);
    let salt = b"request-salt-12345";
    let nonce = b"unique nonce";
    let headers = encryption_headers(salt, nonce, TEST_KEY_ID);
    let mut builder = Request::builder().method("POST").uri("/auth/login");
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let response = app
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn guard_decrypts_valid_payload_and_strips_headers() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let salt = b"request-salt-12345";
    let nonce = b"unique nonce";
    let plaintext = br#"{"email":"user@example.com"}"#;
    let ciphertext = encrypt_body(
        TEST_SECRET.as_bytes(),
        salt,
        nonce,
        &axum::http::Method::POST,
        "/auth/login",
        plaintext,
    );
    let headers = encryption_headers(salt, nonce, TEST_KEY_ID);
    let mut builder = Request::builder().method("POST").uri("/auth/login");
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let response = app
        .oneshot(builder.body(Body::from(ciphertext)).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("x-nvbes-e2ee").is_none());
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    assert_eq!(body.as_ref(), plaintext);
}

#[tokio::test]
async fn guard_rejects_invalid_ciphertext() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let salt = b"request-salt-12345";
    let nonce = b"unique nonce";
    let headers = encryption_headers(salt, nonce, TEST_KEY_ID);
    let mut builder = Request::builder().method("POST").uri("/auth/login");
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let response = app
        .oneshot(builder.body(Body::from(vec![1, 2, 3, 4])).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_rejects_missing_encryption_headers() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("x-nvbes-e2ee", "aes-256-gcm")
                .header("x-nvbes-e2ee-key-id", TEST_KEY_ID)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn guard_rejects_invalid_base64_header() {
    let config = e2ee_config(false);
    let app = e2ee_app(config);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("x-nvbes-e2ee", "aes-256-gcm")
                .header("x-nvbes-e2ee-key-id", TEST_KEY_ID)
                .header("x-nvbes-e2ee-salt", "!!!")
                .header("x-nvbes-e2ee-nonce", "!!!")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
