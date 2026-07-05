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
use crate::region_center_mutations::{RegionFlagInput, flag_residency, record_exception};
use crate::region_center_types::RegionActionResult;

#[derive(Debug, Deserialize)]
struct ResidencyFlagRequest {
    confirm_code: String,
    data_region: String,
    jurisdiction: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ResidencyExceptionRequest {
    confirm_code: String,
    exception_kind: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/residency-flag",
            post(flag_residency_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/exceptions",
            post(record_exception_route),
        )
}

async fn flag_residency_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, target_workspace_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ResidencyFlagRequest>,
) -> Result<Json<RegionActionResult>, AppError> {
    require_region_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "FLAG RESIDENCY",
        target_workspace_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        flag_residency(
            &state.db,
            access,
            workspace_id,
            RegionFlagInput {
                target_workspace_id,
                data_region: request.data_region,
                jurisdiction: request.jurisdiction,
                reason: request.reason,
            },
        )
        .await?,
    ))
}

async fn record_exception_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, target_workspace_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ResidencyExceptionRequest>,
) -> Result<Json<RegionActionResult>, AppError> {
    require_region_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "RECORD REGION EXCEPTION",
        target_workspace_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        record_exception(
            &state.db,
            access,
            workspace_id,
            target_workspace_id,
            request.exception_kind,
            request.reason,
        )
        .await?,
    ))
}

async fn require_region_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_region_authorization(headers)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

fn require_region_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::RegionMutate)
}
