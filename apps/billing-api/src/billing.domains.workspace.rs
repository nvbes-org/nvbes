use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use uuid::Uuid;

use crate::{app::BillingAppState, http::error::AppError};

pub fn router() -> Router<BillingAppState> {
    Router::new()
        .route(
            "/internal/workspaces/{workspaceId}/billing/overview",
            get(get_billing_overview),
        )
        .route(
            "/internal/workspaces/{workspaceId}/billing/usage",
            get(get_billing_usage),
        )
        .route(
            "/internal/workspaces/{workspaceId}/billing/entitlements",
            get(get_entitlements),
        )
}

pub async fn get_billing_overview(
    State(state): State<BillingAppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::BillingOverviewResponse>, AppError> {
    Ok(Json(
        nvbes_billing::fetch_workspace_billing_overview(&state.db, workspace_id).await?,
    ))
}

pub async fn get_billing_usage(
    State(state): State<BillingAppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::BillingUsageResponse>, AppError> {
    Ok(Json(
        nvbes_billing::fetch_workspace_billing_usage(&state.db, workspace_id).await?,
    ))
}

pub async fn get_entitlements(
    State(state): State<BillingAppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<nvbes_billing::types::ProductEntitlementsView>, AppError> {
    Ok(Json(
        nvbes_billing::fetch_workspace_entitlements(&state.db, workspace_id).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_billing_routes_are_registered() {
        let router = router();
        let _ = router;
    }
}
