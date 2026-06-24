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
use crate::operations_center_mutations::{
    replay_export_run, replay_provider_event, resolve_reconciliation_difference,
};
use crate::operations_center_types::OperationsActionResult;

#[derive(Debug, Deserialize)]
struct OperationsActionRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/operations/provider-events/{eventId}/replay",
            post(replay_provider_event_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/operations/export-runs/{exportRunId}/replay",
            post(replay_export_run_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/operations/reconciliation-differences/{differenceId}/resolve",
            post(resolve_reconciliation_difference_route),
        )
}

async fn replay_provider_event_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, event_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<OperationsActionRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(&headers, &request.confirm_code, "REPLAY PROVIDER EVENT")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_provider_event(&state.db, access, workspace_id, event_id, request.reason).await?,
    ))
}

async fn replay_export_run_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, export_run_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<OperationsActionRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(&headers, &request.confirm_code, "REPLAY EXPORT RUN")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_export_run(
            &state.db,
            access,
            workspace_id,
            export_run_id,
            request.reason,
        )
        .await?,
    ))
}

async fn resolve_reconciliation_difference_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, difference_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<OperationsActionRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(&headers, &request.confirm_code, "RESOLVE RECON DIFFERENCE")?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        resolve_reconciliation_difference(
            &state.db,
            access,
            workspace_id,
            difference_id,
            request.reason,
        )
        .await?,
    ))
}

fn require_operations_mutation(
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::OperationsMutate)?;
    require_confirmation(confirm_code, expected_code)
}
