use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_confirmation, require_idempotency_key, require_permission,
};
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;
use crate::risk_decision_center_mutations::{approve_policy, block_policy, resolve_risk_signal};
use crate::risk_decision_center_types::RiskActionResult;

#[derive(Debug, Deserialize)]
struct RiskActionRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/risk/policies/{policyId}/approve",
            post(approve_policy_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/risk/policies/{policyId}/block",
            post(block_policy_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/risk/signals/{signalId}/resolve",
            post(resolve_risk_signal_route),
        )
}

async fn approve_policy_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, policy_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RiskActionRequest>,
) -> Result<Json<RiskActionResult>, AppError> {
    require_risk_mutation(&headers, &request.confirm_code, "APPROVE RISK POLICY")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        approve_policy(&state.db, access, workspace_id, policy_id, request.reason).await?,
    ))
}

async fn block_policy_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, policy_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RiskActionRequest>,
) -> Result<Json<RiskActionResult>, AppError> {
    require_risk_mutation(&headers, &request.confirm_code, "BLOCK RISK POLICY")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        block_policy(&state.db, access, workspace_id, policy_id, request.reason).await?,
    ))
}

async fn resolve_risk_signal_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, signal_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RiskActionRequest>,
) -> Result<Json<RiskActionResult>, AppError> {
    require_risk_mutation(&headers, &request.confirm_code, "RESOLVE RISK SIGNAL")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        resolve_risk_signal(&state.db, access, workspace_id, signal_id, request.reason).await?,
    ))
}

fn require_risk_mutation(
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::RiskMutate)?;
    require_confirmation(confirm_code, expected_code)
}
