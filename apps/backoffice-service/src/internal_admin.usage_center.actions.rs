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
    require_strong_confirmation, require_strong_confirmation_for_value,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;
use crate::usage_center_mutations::{
    UsageActionResult, UsageCorrectionInput, correct_usage, freeze_meter, replay_rollup,
};

#[derive(Debug, Deserialize)]
struct UsageCorrectionRequest {
    confirm_code: String,
    usage_event_id: Option<Uuid>,
    meter_code: String,
    quantity_delta: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct FreezeMeterRequest {
    confirm_code: String,
    meter_code: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ReplayRollupRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/usage/corrections",
            post(correct_usage_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/usage/meters/freeze",
            post(freeze_meter_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/usage/rollups/{rollupId}/replay",
            post(replay_rollup_route),
        )
}

async fn correct_usage_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<UsageCorrectionRequest>,
) -> Result<Json<UsageActionResult>, AppError> {
    require_usage_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "CORRECT USAGE",
        &request.meter_code,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        correct_usage(
            &state.billing_grpc_endpoint,
            &state.db,
            access,
            workspace_id,
            UsageCorrectionInput {
                usage_event_id: request.usage_event_id,
                meter_code: request.meter_code,
                quantity_delta: request.quantity_delta,
                reason: request.reason,
            },
        )
        .await?,
    ))
}

async fn freeze_meter_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<FreezeMeterRequest>,
) -> Result<Json<UsageActionResult>, AppError> {
    require_usage_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "FREEZE METER",
        &request.meter_code,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        freeze_meter(
            &state.billing_grpc_endpoint,
            &state.db,
            access,
            workspace_id,
            request.meter_code,
            request.reason,
        )
        .await?,
    ))
}

async fn replay_rollup_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, rollup_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ReplayRollupRequest>,
) -> Result<Json<UsageActionResult>, AppError> {
    require_usage_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REPLAY ROLLUP",
        rollup_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_rollup(
            &state.billing_grpc_endpoint,
            &state.db,
            access,
            workspace_id,
            rollup_id,
            request.reason,
        )
        .await?,
    ))
}

async fn require_usage_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::UsageMutate)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

async fn require_usage_mutation_for_value(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: &str,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::UsageMutate)?;
    require_strong_confirmation_for_value(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}
