use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::app::BillingState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub fn router() -> Router<BillingState> {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

async fn live() -> Json<HealthResponse> {
    Json(response("alive"))
}

async fn ready(State(state): State<BillingState>) -> (StatusCode, Json<HealthResponse>) {
    let ready = tokio::time::timeout(
        std::time::Duration::from_millis(250),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .is_some();

    if ready {
        (StatusCode::OK, Json(response("ready")))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response("not_ready")))
    }
}

fn response(status: &'static str) -> HealthResponse {
    HealthResponse {
        status,
        service: "nvbes-billing-service",
    }
}
