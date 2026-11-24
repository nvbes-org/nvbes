use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{config::WorkerConfig, health};

fn test_pool() -> sqlx::PgPool {
    let url = std::env::var("NVBES_SECURITY_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://127.0.0.1:1/nvbes_test".into());
    PgPoolOptions::new()
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_lazy(&url)
        .unwrap()
}

#[tokio::test]
async fn healthz_returns_pass() {
    let pool = test_pool();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_live_returns_pass() {
    let pool = test_pool();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readiness_endpoints_work() {
    let pool = test_pool();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    for uri in ["/readyz", "/health/ready"] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::SERVICE_UNAVAILABLE
        );
    }
}

#[tokio::test]
async fn metrics_returns_pass() {
    let pool = test_pool();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_endpoint_with_internal_token_guard() {
    let pool = test_pool();

    let mut config = WorkerConfig::from_env().unwrap();
    config.environment = "production".to_string();
    config.observability_internal_token = Some("worker_secret_token".to_string());
    let router = health::router(pool, &config);

    // 1. Without authorization header -> 401
    let unauth = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    // 2. With wrong token -> 401
    let wrong = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("Authorization", "Bearer wrong_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);

    // 3. With valid token -> 200
    let valid = router
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("Authorization", "Bearer worker_secret_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(valid.status(), StatusCode::OK);
}

#[tokio::test]
async fn root_returns_metadata() {
    let pool = test_pool();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
