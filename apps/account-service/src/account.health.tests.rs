use axum::{body::Body, http::Request};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{app::AccountState, config::AccountConfig, health, metrics};

fn state() -> AccountState {
    let database_url = "postgres://account:account@127.0.0.1:1/account_test";
    let db = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(10))
        .connect_lazy(database_url)
        .unwrap();
    AccountState::new(
        AccountConfig {
            environment: "test".into(),
            database_url: database_url.into(),
            database_max_connections: 1,
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            token_issuer: "http://identity.local".into(),
            token_audience: "nvbes-account-service".into(),
            token_key_id: "identity-key-1".into(),
            token_public_key_pem: "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwEKbtGra6bWscKp5s8i3\njvE0mUrZV54vaDWEfYxnjJYc6TNPBUuHlNUOv44eePZ+TxLQ9wPYSwuhSJIPAX7c\nAkDDdzVJy36lUVTjKGND7PZtgv6ItPQb6yM7YNyM7+QHZXL0fvB5q7O1AasgT+sF\ns0ffePjSyi9QpOww8TqcgePyXN3anUmB8pwoaJQfJOOLE1sJ3zsfw+n3nG+31Lpg\nDQBwYQWrKRRH/R7aYRFoZu1ZRFINfYXVmOGUuehcYkAprNs1dte6szKyyc2zhUJi\nTUez2NsFzgzx1Q1ssxYOMbQcVqNjux5l2aC9i5VcPi7gRHWGKiN77sSjnz90vqSu\nYwIDAQAB\n-----END PUBLIC KEY-----\n".into(),
            metrics_token: "development-account-metrics-token-value".into(),
            sentry_dsn: None,
            sentry_traces_sample_rate: 0.1,
            otlp_endpoint: None,
            otlp_authorization_header: None,
        },
        db,
    )
    .unwrap()
}

#[tokio::test]
async fn liveness_is_shallow_but_readiness_requires_database() {
    let state = state();
    state.db.close().await;
    let app = health::router(state);
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
async fn metrics_require_exact_bearer_token() {
    let state = state();
    let app = metrics::router(state);
    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), axum::http::StatusCode::UNAUTHORIZED);

    let authorized = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header(
                    "authorization",
                    "Bearer development-account-metrics-token-value",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), axum::http::StatusCode::OK);
}
