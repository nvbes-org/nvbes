use axum::{Json, Router, routing::get};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub fn router() -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

async fn live() -> Json<HealthResponse> {
    response("alive")
}

async fn ready() -> Json<HealthResponse> {
    response("ready")
}

fn response(status: &'static str) -> Json<HealthResponse> {
    Json(HealthResponse {
        status,
        service: "nvbes-identity-service",
    })
}

#[cfg(test)]
#[path = "identity.health.tests.rs"]
mod tests;
