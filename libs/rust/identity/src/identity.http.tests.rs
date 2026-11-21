use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use tower::ServiceExt;

use super::{HttpError, router};
use crate::health::tests::state as lazy_state;

#[test]
fn http_error_maps_to_expected_status_codes() {
    let bad = HttpError::BadRequest("bad".into()).into_response();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

    let unauthorized = HttpError::Unauthorized("nope".into()).into_response();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let forbidden = HttpError::Forbidden("closed".into()).into_response();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let conflict = HttpError::Conflict("taken".into()).into_response();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let internal = HttpError::InternalServerError("boom".into()).into_response();
    assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn register_is_forbidden_when_public_signup_is_closed() {
    let mut state = lazy_state();
    let mut config = (*state.config).clone();
    config.public_signup_enabled = false;
    state.config = std::sync::Arc::new(config);

    let response = router(&state)
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

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
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

#[tokio::test]
async fn login_route_is_present() {
    let response = router(&lazy_state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
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
async fn logout_route_is_present() {
    let response = router(&lazy_state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
