use std::time::Instant;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::app::TrustRiskState;

#[derive(Debug, Serialize)]
pub(crate) struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub fn router(state: TrustRiskState) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .with_state(state)
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "alive",
        service: "nvbes-trust-risk-service",
    })
}

pub(crate) async fn ready(
    State(state): State<TrustRiskState>,
) -> (StatusCode, Json<HealthResponse>) {
    let database_ready = tokio::time::timeout(
        std::time::Duration::from_millis(250),
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM trust_risk_rule_sets WHERE state = 'active')",
        )
        .fetch_one(&state.db),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or(false);
    let heartbeat = *state.projection_heartbeat.read().await;
    let projection_ready = heartbeat_current(
        heartbeat,
        Instant::now(),
        state.config.projection_heartbeat_max_age,
    );
    let (status, label) = if database_ready && projection_ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not_ready")
    };
    (
        status,
        Json(HealthResponse {
            status: label,
            service: "nvbes-trust-risk-service",
        }),
    )
}

fn heartbeat_current(
    heartbeat: Option<Instant>,
    now: Instant,
    max_age: std::time::Duration,
) -> bool {
    heartbeat.is_some_and(|heartbeat| now.saturating_duration_since(heartbeat) <= max_age)
}

#[cfg(test)]
#[path = "trust_risk.health.tests.rs"]
mod tests;
