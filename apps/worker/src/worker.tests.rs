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

    let router = health::router().with_state(pool);

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
async fn root_returns_metadata() {
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

    let router = health::router().with_state(pool);

    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
