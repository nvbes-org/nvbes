use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use nvbes_core::http::internal_observability::{
    InternalObservabilityConfig, internal_observability_guard_with_config,
};
use nvbes_observability::metrics::HttpMetrics;
use serde::Serialize;
use sqlx::PgPool;

use crate::config::WorkerConfig;

#[derive(Debug, Serialize)]
pub struct WorkerHealthStatus {
    pub status: &'static str,
    pub worker: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct WorkerReadinessStatus {
    pub status: &'static str,
    pub database: &'static str,
}

pub fn router(pool: PgPool, config: &WorkerConfig) -> Router {
    let http_metrics = HttpMetrics::new();
    let observability_config = InternalObservabilityConfig::new(
        &config.environment,
        config.observability_internal_token.as_deref(),
    );

    let metrics_router = Router::new()
        .route("/metrics", get(nvbes_observability::metrics_handler))
        .layer(axum::middleware::from_fn_with_state(
            observability_config,
            internal_observability_guard_with_config,
        ))
        .with_state(http_metrics);

    let health_router = Router::new()
        .route("/healthz", get(healthz_handler))
        .route("/health/live", get(healthz_handler))
        .route("/readyz", get(readyz_handler))
        .route("/health/ready", get(readyz_handler))
        .route("/", get(root_handler))
        .with_state(pool);

    health_router.merge(metrics_router)
}

async fn healthz_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(WorkerHealthStatus {
            status: "pass",
            worker: "nvbes-worker",
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

async fn readyz_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(WorkerReadinessStatus {
                status: "pass",
                database: "connected",
            }),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(WorkerReadinessStatus {
                status: "fail",
                database: "disconnected",
            }),
        ),
    }
}

async fn root_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "nvbes-worker",
        "description": "NVBES Unified Background Worker",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
