use axum::{Json, Router, extract::State, routing::post};
use serde::Serialize;

use crate::{
    app::AppState,
    domains::billing::entitlements::{
        BillingEntitlementChangedEvent, drive_entitlement_projection_from_event,
        persist_entitlement_projection,
    },
    http::error::AppError,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/internal/billing/entitlement-events",
        post(project_entitlement_event),
    )
}

#[derive(Debug, Serialize)]
pub struct EntitlementProjectionResponse {
    pub projected: bool,
}

pub async fn project_entitlement_event(
    State(state): State<AppState>,
    Json(event): Json<BillingEntitlementChangedEvent>,
) -> Result<Json<EntitlementProjectionResponse>, AppError> {
    let projection = drive_entitlement_projection_from_event(event);
    persist_entitlement_projection(&state.db, projection).await?;
    Ok(Json(EntitlementProjectionResponse { projected: true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entitlement_projection_route_is_registered() {
        let router = router();
        let _ = router;
    }
}
