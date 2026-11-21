use axum::{Json, Router, http::StatusCode, routing::get};
use serde::Serialize;

use crate::state::EmailWorkerState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub fn router(state: EmailWorkerState) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .with_state(state)
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "alive",
        service: "nvbes-email-worker",
    })
}

async fn ready() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ready",
            service: "nvbes-email-worker",
        }),
    )
}

#[cfg(test)]
#[path = "email.worker.health.tests.rs"]
mod tests;
