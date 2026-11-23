use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{config::WorkerConfig, health};

#[tokio::test]
async fn config_loads_defaults() {
    let config = WorkerConfig::from_env();
    assert!(config.is_ok());
    let cfg = config.unwrap();
    assert_eq!(cfg.environment, "development");
    assert_eq!(cfg.database_min_connections, 0);
    assert_eq!(cfg.poll_interval_secs, 5);
    assert_eq!(cfg.privacy_interval_secs, 60);
}

#[tokio::test]
async fn healthz_returns_pass() {
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

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
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

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
async fn readiness_endpoints_fail_when_database_is_unreachable() {
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    for uri in ["/readyz", "/health/ready"] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}

#[tokio::test]
async fn metrics_returns_pass() {
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

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
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

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
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

    let config = WorkerConfig::from_env().unwrap();
    let router = health::router(pool, &config);

    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
