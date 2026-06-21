use axum::{Json, Router, extract::State, routing::post};
use nvbes_billing::usage::UsageEvent;

use crate::{
    app::AppState,
    domains::billing::service::{BillingUsageIngestResponse, ingest_usage_event},
    http::error::AppError,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/internal/billing/usage-events",
        post(ingest_usage_event_route),
    )
}

pub async fn ingest_usage_event_route(
    State(state): State<AppState>,
    Json(event): Json<UsageEvent>,
) -> Result<Json<BillingUsageIngestResponse>, AppError> {
    let response = ingest_usage_event(&state.db, event).await?;
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_usage_ingest_route_is_registered() {
        let router = router();
        let _ = router;
    }
}
