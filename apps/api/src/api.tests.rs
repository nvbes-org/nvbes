use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{IdentityRuntime, build_router, config::ApiConfig};

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

/// Identity always runs on development fallbacks here, whatever environment a
/// test gives the API configuration.
fn app(pool: sqlx::PgPool, config: &ApiConfig) -> axum::Router {
    let identity = IdentityRuntime::from_lookup("test", &|_| None).expect("development identity");
    build_router(pool, config, &identity).expect("api router")
}

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
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

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
async fn health_live_returns_service_name() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "nvbes-api");
    assert_eq!(body["status"], "pass");
}

#[tokio::test]
async fn root_returns_metadata() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn openapi_spec_is_accessible() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

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

#[tokio::test]
async fn account_routes_are_mounted_and_require_a_token() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    for uri in ["/api/v1/profile", "/api/v1/consents"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn run_migrations_fails_closed_when_database_is_unreachable() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://127.0.0.1:1/unreachable")
        .unwrap();
    let result = crate::migrations::run_migrations(&pool).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn response_contains_observability_headers() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

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
    assert!(response.headers().contains_key("x-request-id"));
    assert!(response.headers().contains_key("traceparent"));
    assert!(response.headers().contains_key("server-timing"));
}

#[tokio::test]
async fn metrics_endpoint_returns_prometheus_metrics() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    let response = app
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
    let mut config = ApiConfig::from_env().unwrap();
    config.environment = "production".to_string();
    config.observability_internal_token = Some("api_secret_token".to_string());
    let app = app(pool, &config);

    // 1. Without authorization header -> 401
    let unauth = app
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
    let wrong = app
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
    let valid = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("Authorization", "Bearer api_secret_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(valid.status(), StatusCode::OK);
}

#[tokio::test]
async fn catch_panic_layer_recovers_from_panic() {
    async fn panic_handler() -> &'static str {
        panic!("test handler panic");
    }

    let layer = tower_http::catch_panic::CatchPanicLayer::new();
    let router = axum::Router::new()
        .route("/panic", axum::routing::get(panic_handler))
        .layer(layer);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/panic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn swagger_ui_and_openapi_spec_are_accessible() {
    let spec = crate::openapi::openapi_spec();
    assert_eq!(spec.info.title, "NVBES Unified Monolith API");

    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 2048).await.unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("NVBES API Documentation"));
}

#[tokio::test]
async fn readiness_endpoints_work() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = app(pool, &config);

    for uri in ["/readyz", "/health/ready"] {
        let response = app
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

#[test]
fn config_error_formatting() {
    assert_eq!(
        format!("{}", crate::config::ConfigError::Missing("DATABASE_URL")),
        "missing required configuration: DATABASE_URL"
    );
    assert_eq!(
        format!("{}", crate::config::ConfigError::Invalid("PORT")),
        "invalid configuration: PORT"
    );
}

#[path = "api.cors.tests.rs"]
mod cors;
#[path = "api.oauth.tests.rs"]
mod oauth;
