#![allow(unused_imports)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use sqlx::PgPool;
use tower::ServiceExt;

use super::{HttpError, normalize_email, router, validate_password};
use crate::health::tests::state as lazy_state;

#[test]
fn normalize_email_accepts_and_lowercases_valid_addresses() {
    assert_eq!(
        normalize_email(" Person@Example.COM ").unwrap(),
        "person@example.com"
    );
}

#[test]
fn normalize_email_rejects_invalid_shapes() {
    for email in [
        "",
        "plain",
        "@missing-local.com",
        "local@",
        "a@b",
        "@.",
        "user@",
    ] {
        assert!(
            matches!(normalize_email(email), Err(HttpError::BadRequest(_))),
            "{email}"
        );
    }
    assert!(normalize_email(&format!("{}@example.com", "a".repeat(320))).is_err());
}

#[test]
fn validate_password_enforces_length_bounds() {
    assert!(validate_password("short").is_err());
    assert!(validate_password("long-enough-password").is_ok());
    assert!(validate_password(&"x".repeat(12)).is_ok());
    assert!(validate_password(&"x".repeat(1024)).is_ok());
    assert!(validate_password(&"x".repeat(1025)).is_err());
}

#[test]
fn http_error_maps_to_expected_status_codes() {
    let bad = HttpError::BadRequest("bad".into()).into_response();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

    let unauthorized = HttpError::Unauthorized("nope".into()).into_response();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let internal = HttpError::InternalServerError("boom".into()).into_response();
    assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn register_rejects_invalid_json_payloads_without_database() {
    let app = router(&lazy_state());
    let missing_body = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_body.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bad_email = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"not-an-email","password":"long-enough-password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_email.status(), StatusCode::BAD_REQUEST);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn register_and_login_round_trip(pool: PgPool) {
    use crate::{app::IdentityState, config::IdentityConfig};

    let state = IdentityState::new(
        IdentityConfig {
            environment: "test".into(),
            sentry_dsn: None,
            sentry_traces_sample_rate: 0.1,
            otlp_endpoint: None,
            otlp_authorization_header: None,
            metrics_token: "identity-metrics-test-token-32-characters".into(),
            database_url: "postgres://unused".into(),
            database_max_connections: 1,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            mfa_encryption_key: [2; 32],
            mfa_key_version: 1,
            mfa_previous_encryption_key: None,
            mfa_previous_key_version: None,
            token_issuer: "http://127.0.0.1:0".into(),
        },
        pool,
    );
    let app = router(&state);
    let email = format!("user-{}@example.com", uuid::Uuid::new_v4().simple());
    let password = "long-enough-password";

    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"{password}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::CREATED);

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"{password}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);

    let bad_login = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"wrong-password-xx"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_login.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_rejects_invalid_email_shape_without_database() {
    let app = router(&lazy_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"not-an-email","password":"long-enough-password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn register_rejects_short_password() {
    let app = router(&lazy_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"ok@example.com","password":"short"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
