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
use crate::communications_center_mutations::{replay_email, suppress_email, unsuppress_email};
use crate::communications_center_types::CommunicationsActionResult;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct CommunicationsReasonRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct EmailSuppressionRequest {
    confirm_code: String,
    email: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/communications/emails/{messageId}/replay",
            post(replay_email_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/communications/suppressions",
            post(suppress_email_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/communications/suppressions/remove",
            post(unsuppress_email_route),
        )
}

async fn replay_email_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, message_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CommunicationsReasonRequest>,
) -> Result<Json<CommunicationsActionResult>, AppError> {
    require_communications_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "REPLAY EMAIL",
        message_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_email(
            &state.db,
            &*state.email_operations,
            access,
            workspace_id,
            message_id,
            request.reason,
        )
        .await?,
    ))
}

async fn suppress_email_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<EmailSuppressionRequest>,
) -> Result<Json<CommunicationsActionResult>, AppError> {
    require_communications_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "SUPPRESS EMAIL",
        workspace_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        suppress_email(
            &state.db,
            &*state.email_operations,
            access,
            workspace_id,
            request.email,
            request.reason,
        )
        .await?,
    ))
}

async fn unsuppress_email_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<EmailSuppressionRequest>,
) -> Result<Json<CommunicationsActionResult>, AppError> {
    require_communications_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "UNSUPPRESS EMAIL",
        workspace_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        unsuppress_email(
            &state.db,
            &*state.email_operations,
            access,
            workspace_id,
            request.email,
            request.reason,
        )
        .await?,
    ))
}

async fn require_communications_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::CommunicationsMutate)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}
