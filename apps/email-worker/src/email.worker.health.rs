use std::time::Duration;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::{database, state::EmailWorkerState};

const MAX_DISPATCHER_HEARTBEAT_AGE: Duration = Duration::from_secs(10);

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

async fn ready(State(state): State<EmailWorkerState>) -> (StatusCode, Json<HealthResponse>) {
    let database_ready = database::database_now(&state.db).await.is_ok();
    let dispatcher_ready = state.dispatcher_is_current(MAX_DISPATCHER_HEARTBEAT_AGE);
    let (status, label) = if database_ready && dispatcher_ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not_ready")
    };
    (
        status,
        Json(HealthResponse {
            status: label,
            service: "nvbes-email-worker",
        }),
    )
}

#[cfg(test)]
#[path = "email.worker.health.tests.rs"]
mod tests;
