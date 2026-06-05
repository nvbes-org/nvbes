use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::audit::AuditRecordInput,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use crate::domains::privacy::service::PrivacyRequestResponse;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/privacy/export",
            post(export_workspace_data),
        )
        .route(
            "/workspaces/{workspaceId}/privacy/delete",
            post(delete_workspace),
        )
}

#[derive(Deserialize, ToSchema)]
struct DeleteWorkspaceRequest {
    confirmation: String,
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/privacy/export",
    tag = "privacy",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Workspace export requested", body = PrivacyRequestResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn export_workspace_data(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<PrivacyRequestResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ExportWorkspaceData,
        ResourceContext::default(),
    )
    .await?;

    let ip = client_ip(&headers);
    let user_agent = user_agent(&headers);

    crate::domains::audit::record_event(
        &state.db,
        AuditRecordInput {
            workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "workspace.export_requested",
            target_type: "workspace",
            target_id: Some(workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "source": "privacy.workspace_export"
            }),
        },
    )
    .await?;

    let result =
        crate::domains::privacy::request_workspace_export(&state.db, &state.redis, &access).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/privacy/delete",
    tag = "privacy",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = DeleteWorkspaceRequest,
    responses(
        (status = 200, description = "Workspace deletion requested", body = PrivacyRequestResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn delete_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<DeleteWorkspaceRequest>,
) -> Result<Json<PrivacyRequestResponse>, AppError> {
    if request.confirmation != "DELETE_WORKSPACE" {
        return Err(AppError::bad_request(
            "confirmation_required",
            "Workspace deletion requires confirmation value DELETE_WORKSPACE.",
        ));
    }

    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::DeleteWorkspace,
        ResourceContext::default(),
    )
    .await?;

    let ip = client_ip(&headers);
    let user_agent = user_agent(&headers);

    crate::domains::audit::record_event(
        &state.db,
        AuditRecordInput {
            workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "workspace.delete_requested",
            target_type: "workspace",
            target_id: Some(workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "source": "privacy.workspace_delete"
            }),
        },
    )
    .await?;

    let result =
        crate::domains::privacy::request_workspace_delete(&state.db, &state.redis, &access).await?;
    Ok(Json(result))
}
