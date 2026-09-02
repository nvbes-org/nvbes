use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::app::IdentityState;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub fn router(state: IdentityState) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .with_state(state)
}

async fn live() -> Json<HealthResponse> {
    Json(response("alive"))
}

async fn ready(State(state): State<IdentityState>) -> (StatusCode, Json<HealthResponse>) {
    let database_ready = tokio::time::timeout(
        std::time::Duration::from_millis(250),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .is_some();
    let (status, label) = if database_ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not_ready")
    };
    (status, Json(response(label)))
}

fn response(status: &'static str) -> HealthResponse {
    HealthResponse {
        status,
        service: "nvbes-identity-service",
    }
}

#[cfg(test)]
#[path = "identity.health.tests.rs"]
pub(crate) mod tests;
