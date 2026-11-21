use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{build_router, config::ApiConfig};

#[tokio::test]
async fn config_loads_defaults() {
    let config = ApiConfig::from_env();
    assert!(config.is_ok());
    let cfg = config.unwrap();
    assert_eq!(cfg.environment, "development");
    assert_eq!(cfg.database_min_connections, 0);
}

#[tokio::test]
async fn healthz_returns_pass() {
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    let response = app
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

    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn openapi_spec_is_accessible() {
    let pool = PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap();

    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
