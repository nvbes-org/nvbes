use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::{
    build_router,
    config::ApiConfig,
    ports::{
        AccountPort, IdentityPort, InProcessAccountAdapter, InProcessIdentityAdapter, PortError,
    },
};

fn test_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .min_connections(0)
        .connect_lazy("postgres://localhost/nvbes_test")
        .unwrap()
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
async fn health_live_returns_service_name() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

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
    let app = build_router(pool, &config);

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

#[tokio::test]
async fn oidc_discovery_is_accessible() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/openid-configuration")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["issuer"], "http://localhost:8080");
    assert_eq!(body["jwks_uri"], "http://localhost:8080/oauth/jwks");
}

#[tokio::test]
async fn oidc_jwks_is_accessible() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/oauth/jwks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["keys"].is_array());
}

#[tokio::test]
async fn consent_routes_are_mounted() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/consents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Route exists and is mounted (returns 200 or handles request, not 404)
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn in_process_identity_port_rejects_invalid_token() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let (_router, token_svc) = crate::identity::build_router(pool, &config);
    let token_service = token_svc.expect("TokenService must be available in dev/test");

    let adapter = InProcessIdentityAdapter::new(token_service);
    let result = adapter.introspect_token("invalid.jwt.token", "nvbes-account-service");

    assert!(matches!(result, Err(PortError::Unauthorized(_))));
}

#[tokio::test]
async fn in_process_account_port_handles_database_error() {
    let pool = test_pool();
    let adapter = InProcessAccountAdapter::new(pool);
    let result = adapter
        .is_team_member(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
        .await;
    assert!(matches!(result, Err(PortError::Internal(_))));
}

#[tokio::test]
async fn run_migrations_fails_closed_when_database_is_unreachable() {
    let pool = test_pool();
    let result = crate::migrations::run_migrations(&pool).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn response_contains_observability_headers() {
    let pool = test_pool();
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
    assert!(response.headers().contains_key("x-request-id"));
    assert!(response.headers().contains_key("traceparent"));
    assert!(response.headers().contains_key("server-timing"));
}

#[tokio::test]
async fn metrics_endpoint_returns_prometheus_metrics() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

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
    let app = build_router(pool, &config);

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
    let app = build_router(pool, &config);

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
    let app = build_router(pool, &config);

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
fn port_error_and_principal_contract() {
    assert_eq!(
        format!("{}", PortError::Unauthorized("denied".into())),
        "unauthorized: denied"
    );
    assert_eq!(
        format!("{}", PortError::InvalidArgument("bad".into())),
        "invalid argument: bad"
    );
    assert_eq!(
        format!("{}", PortError::NotFound("missing".into())),
        "not found: missing"
    );
    assert_eq!(
        format!("{}", PortError::Internal("crash".into())),
        "internal port error: crash"
    );

    let id = uuid::Uuid::new_v4();
    let principal = crate::ports::AuthenticatedPrincipal {
        id,
        scopes: vec!["account:read".into(), "account:write".into()],
        client_id: Some("nvbes-web".into()),
        step_up: true,
    };
    assert_eq!(principal.id, id);
    assert_eq!(principal.scopes.len(), 2);
    assert_eq!(principal.client_id.as_deref(), Some("nvbes-web"));
    assert!(principal.step_up);
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

#[tokio::test]
async fn oauth_routes_are_mounted_on_api() {
    let pool = test_pool();
    let config = ApiConfig::from_env().unwrap();
    let app = build_router(pool, &config);

    // /oauth/authorize should not return 404
    let auth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(auth.status(), StatusCode::NOT_FOUND);

    // /oauth/token should not return 404
    let token = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(token.status(), StatusCode::NOT_FOUND);
}
