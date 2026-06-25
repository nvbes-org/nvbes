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
use crate::developer_center_mutations::{approve_marketplace_app, revoke_client, rotate_secret};
use crate::developer_center_types::DeveloperActionResult;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct DeveloperReasonRequest {
    confirm_code: String,
    reason: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/admin/developer/clients/{clientId}/revoke",
            post(revoke_client_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/developer/clients/{clientId}/rotate-secret",
            post(rotate_secret_route),
        )
        .route(
            "/workspaces/{workspaceId}/admin/developer/marketplace-apps/{appId}/approve",
            post(approve_marketplace_app_route),
        )
}

async fn revoke_client_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, client_id)): Path<(Uuid, String)>,
    Json(request): Json<DeveloperReasonRequest>,
) -> Result<Json<DeveloperActionResult>, AppError> {
    require_developer_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "REVOKE CLIENT",
        &client_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        revoke_client(&state.db, access, workspace_id, client_id, request.reason).await?,
    ))
}

async fn rotate_secret_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, client_id)): Path<(Uuid, String)>,
    Json(request): Json<DeveloperReasonRequest>,
) -> Result<Json<DeveloperActionResult>, AppError> {
    require_developer_mutation_for_value(
        &state.db,
        &headers,
        &request.confirm_code,
        "ROTATE SECRET",
        &client_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        rotate_secret(&state.db, access, workspace_id, client_id, request.reason).await?,
    ))
}

async fn approve_marketplace_app_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, app_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<DeveloperReasonRequest>,
) -> Result<Json<DeveloperActionResult>, AppError> {
    require_developer_mutation(
        &state.db,
        &headers,
        &request.confirm_code,
        "APPROVE MARKETPLACE APP",
        app_id,
    )
    .await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        approve_marketplace_app(&state.db, access, workspace_id, app_id, request.reason).await?,
    ))
}

async fn require_developer_mutation(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::DeveloperMutate)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}

async fn require_developer_mutation_for_value(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: &str,
) -> Result<(), AppError> {
    require_operator_permission_headers(headers, BackofficePermission::DeveloperMutate)?;
    require_strong_confirmation_for_value(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)?;
    require_operator_role_grant(db, headers).await
}
