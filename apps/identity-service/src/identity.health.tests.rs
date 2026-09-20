use axum::{body::Body, http::Request};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{app::IdentityState, config::IdentityConfig};

pub(crate) fn state() -> IdentityState {
    let database_url = "postgres://identity:identity@127.0.0.1:1/identity_test";
    let db = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(10))
        .connect_lazy(database_url)
        .unwrap();
    IdentityState::new(
        IdentityConfig {
            environment: "test".into(),
            sentry_dsn: None,
            sentry_traces_sample_rate: 0.1,
            otlp_endpoint: None,
            otlp_authorization_header: None,
            metrics_token: "identity-metrics-test-token-32-characters".into(),
            database_url: database_url.into(),
            database_max_connections: 1,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            mfa_encryption_key: [2; 32],
            mfa_key_version: 1,
            mfa_previous_encryption_key: None,
            mfa_previous_key_version: None,
            token_issuer: "http://127.0.0.1:0".into(),
        },
        db,
    )
}

#[tokio::test]
async fn liveness_is_shallow_but_readiness_requires_the_database() {
    let state = state();
    // Model an unavailable database without opening a real network connection.
    state.db.close().await;
    assert!(matches!(
        state.db.acquire().await,
        Err(sqlx::Error::PoolClosed)
    ));
    let app = crate::health::router(&state);
    let live = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(live.status().is_success());
    let ready = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ready.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn public_identity_routes_are_exposed() {
    let response = crate::http::router(&state())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Should return 415 Unsupported Media Type due to missing JSON body, not 404
    assert_eq!(response.status(), axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE);
}