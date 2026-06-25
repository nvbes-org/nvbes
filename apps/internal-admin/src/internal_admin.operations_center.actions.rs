use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use chrono::{DateTime, Utc};
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
use crate::operations_center_mutations::{
    replay_export_run, replay_provider_event, resolve_reconciliation_difference,
};
use crate::operations_center_types::OperationsActionResult;
use crate::operations_center_workflow_mutations::{
    replay_job_run, schedule_maintenance_window, update_incident_state,
};

#[derive(Debug, Deserialize)]
struct OperationsActionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct IncidentStateRequest {
    confirm_code: String,
    reason: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct MaintenanceWindowRequest {
    confirm_code: String,
    reason: String,
    title: String,
    scheduled_start_at: DateTime<Utc>,
    scheduled_end_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/operations/incidents/{incidentId}/state",
            post(update_incident_state_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/operations/maintenance-windows",
            post(schedule_maintenance_window_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/operations/job-runs/{jobRunId}/replay",
            post(replay_job_run_route),
        )
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

async fn update_incident_state_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, incident_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<IncidentStateRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "UPDATE INCIDENT",
        incident_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        update_incident_state(
            &state.db,
            access,
            workspace_id,
            incident_id,
            request.status,
            request.reason,
        )
        .await?,
    ))
}

async fn schedule_maintenance_window_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<MaintenanceWindowRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "SCHEDULE MAINTENANCE",
        workspace_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        schedule_maintenance_window(
            &state.db,
            access,
            workspace_id,
            request.title,
            request.scheduled_start_at,
            request.scheduled_end_at,
            request.reason,
        )
        .await?,
    ))
}

async fn replay_job_run_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, job_run_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<OperationsActionRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REPLAY JOB RUN",
        job_run_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_job_run(&state.db, access, workspace_id, job_run_id, request.reason).await?,
    ))
}

async fn replay_provider_event_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, event_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<OperationsActionRequest>,
) -> Result<Json<OperationsActionResult>, AppError> {
    require_operations_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REPLAY PROVIDER EVENT",
        event_id,
    )
    .await?;
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
    require_operations_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REPLAY EXPORT RUN",
        export_run_id,
    )
    .await?;
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
    require_operations_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "RESOLVE RECON DIFFERENCE",
        difference_id,
    )
    .await?;
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

async fn require_operations_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::OperationsMutate)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}
