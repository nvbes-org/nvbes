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
    BackofficePermission, require_operator_permission_headers, require_operator_role_grant,
    require_strong_confirmation,
};
use crate::backoffice_dual_control::require_dual_control;
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
    require_risk_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "APPROVE RISK POLICY",
        policy_id,
    )
    .await?;
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
    require_risk_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "BLOCK RISK POLICY",
        policy_id,
    )
    .await?;
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
    require_risk_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "RESOLVE RISK SIGNAL",
        signal_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        resolve_risk_signal(&state.db, access, workspace_id, signal_id, request.reason).await?,
    ))
}

async fn require_risk_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_risk_authorization(headers)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

fn require_risk_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::RiskMutate)
}
