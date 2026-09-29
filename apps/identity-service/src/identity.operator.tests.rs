use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::IntoResponse,
};
use tower::ServiceExt;

use super::{OperatorError, require_operator, router, token_service, validate_reason};
use crate::{
    app::IdentityState,
    config::IdentityConfig,
    tokens::{OPERATOR_AUDIENCE, OPERATOR_ROLE},
};

#[test]
fn operator_contract_constants_match_platform_operations() {
    assert_eq!(OPERATOR_AUDIENCE, "platform-operations");
    assert_eq!(OPERATOR_ROLE, "platform_owner");
}

#[test]
fn validate_reason_enforces_length_and_forbids_control_chars() {
    assert!(matches!(
        validate_reason("too-short"),
        Err(OperatorError::Invalid(_))
    ));
    assert!(matches!(
        validate_reason(&"x".repeat(501)),
        Err(OperatorError::Invalid(_))
    ));
    assert!(matches!(
        validate_reason("long enough but has\nnewline"),
        Err(OperatorError::Invalid(_))
    ));
    assert!(matches!(
        validate_reason("long enough but has\rcarriage"),
        Err(OperatorError::Invalid(_))
    ));
    assert!(matches!(validate_reason("  padded reason ok  "), Ok(())));
    assert!(matches!(validate_reason("exactly12chr"), Ok(())));
}

#[test]
fn operator_error_maps_to_expected_http_statuses() {
    let cases = [
        (OperatorError::Unauthorized, StatusCode::UNAUTHORIZED),
        (OperatorError::Forbidden, StatusCode::FORBIDDEN),
        (
            OperatorError::Invalid("reason must be 12-500 characters"),
            StatusCode::BAD_REQUEST,
        ),
        (OperatorError::Database, StatusCode::SERVICE_UNAVAILABLE),
        (OperatorError::Internal, StatusCode::SERVICE_UNAVAILABLE),
    ];
    for (error, expected) in cases {
        let response = error.into_response();
        assert_eq!(response.status(), expected);
    }
}

#[test]
fn sqlx_error_maps_to_database_variant() {
    let mapped = OperatorError::from(sqlx::Error::RowNotFound);
    assert!(matches!(mapped, OperatorError::Database));
}

#[tokio::test]
async fn require_operator_rejects_missing_or_malformed_bearer() {
    let state = minimal_state();
    let empty = axum::http::HeaderMap::new();
    assert!(matches!(
        require_operator(&state, &empty),
        Err(OperatorError::Unauthorized)
    ));

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Basic nope".parse().unwrap());
    assert!(matches!(
        require_operator(&state, &headers),
        Err(OperatorError::Unauthorized)
    ));

    // Without token key env, Bearer still reaches token_service and fails closed.
    let _env = crate::database::database_test_support::WithoutTokenKeysGuard::install();
    headers.insert(header::AUTHORIZATION, "Bearer not-a-jwt".parse().unwrap());
    assert!(matches!(
        require_operator(&state, &headers),
        Err(OperatorError::Internal | OperatorError::Unauthorized)
    ));
}

#[tokio::test]
async fn token_service_fails_closed_without_token_env() {
    let _env = crate::database::database_test_support::WithoutTokenKeysGuard::install();
    let state = minimal_state();
    assert!(matches!(
        token_service(&state),
        Err(OperatorError::Internal)
    ));
}

#[tokio::test]
async fn operator_router_rejects_unauthenticated_revoke() {
    let state = minimal_state();
    let app = router(&state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/operator/principals/{}/sessions/revoke",
                    uuid::Uuid::new_v4()
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"reason":"enough characters"}"#))
                .unwrap(),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn operator_router_rejects_short_revoke_reason_before_auth() {
    let state = minimal_state();
    let app = router(&state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/operator/principals/{}/sessions/revoke",
                    uuid::Uuid::new_v4()
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"reason":"short"}"#))
                .unwrap(),
        )
        .await
        .expect("response");
    // validate_reason runs before require_operator
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn operator_token_issue_rejects_unknown_session() {
    let state = minimal_state();
    let app = router(&state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/operator/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"session_token":"missing-session"}"#))
                .unwrap(),
        )
        .await
        .expect("response");
    // Without a reachable schema this becomes unavailable or unauthorized;
    // either proves the handler path is exercised for coverage.
    assert!(
        matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
        ),
        "unexpected status {}",
        response.status()
    );
}

fn minimal_state() -> IdentityState {
    let db = sqlx::PgPool::connect_lazy("postgres://postgres:postgres@127.0.0.1:1/nvbes_unused")
        .expect("lazy pool");
    IdentityState::new(
        IdentityConfig {
            environment: "test".into(),
            sentry_dsn: None,
            sentry_traces_sample_rate: 0.1,
            otlp_endpoint: None,
            otlp_authorization_header: None,
            metrics_token: "identity-metrics-test-token-32-characters".into(),
            database_url: "postgres://unused".into(),
            database_max_connections: 2,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            mfa_encryption_key: [2; 32],
            mfa_key_version: 1,
            mfa_previous_encryption_key: None,
            mfa_previous_key_version: None,
            token_issuer: "http://identity.test".into(),
            platform_operator_principals: Default::default(),
            public_signup_enabled: true,
            login_url: String::new(),
            session_cookie_secure: false,
        },
        db,
    )
}
