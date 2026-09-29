use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{config::BillingWorkerConfig, state::BillingWorkerState};

#[tokio::test]
async fn metrics_requires_valid_bearer_token() {
    // Build config explicitly: from_env() races with config EnvGuard tests that
    // temporarily set NVBES_EMAIL_* and would make EmailClient::connect fail.
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "http://localhost:3000".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: Some("secret_metrics_token".into()),
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };

    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();

    let state = BillingWorkerState::new(config, db).await.unwrap();
    let app = super::router(state);

    // 1. Missing header
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Wrong token
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("authorization", "Bearer wrong_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Valid token
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("authorization", "Bearer secret_metrics_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_rejects_when_token_is_unset() {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "http://localhost:3000".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: None,
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };

    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();

    let state = BillingWorkerState::new(config, db).await.unwrap();
    let app = super::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("authorization", "Bearer anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn metrics_rejects_when_token_is_empty() {
    let config = BillingWorkerConfig {
        environment: "development".into(),
        database_url: "postgres://localhost/unused".into(),
        http_bind_addr: "127.0.0.1:8080".parse().unwrap(),
        app_url: "http://localhost:3000".into(),
        email_grpc_endpoint: None,
        email_token: None,
        billing_grpc_endpoint: None,
        billing_grpc_token: None,
        metrics_token: Some(String::new()),
        dispatch_mode: crate::config::DispatchMode::InMemory,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    };

    let db = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();

    let state = BillingWorkerState::new(config, db).await.unwrap();
    let app = super::router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
