use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{
        ResourceContext, WorkspaceAction, authorize_workspace_action, resource_context_for_object,
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use crate::domains::files::service::ObjectResponse;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}/trash",
            post(trash_object),
        )
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}/restore",
            post(restore_object),
        )
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/objects/{objectId}/trash",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    responses(
        (status = 200, description = "Object trashed", body = ObjectResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn trash_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ObjectResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;
    let resource = resource_context_for_object(
        &state.db,
        workspace_id,
        access.auth.user_id,
        access.auth.principal_id,
        object_id,
    )
    .await?;
    authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::TrashObject,
        resource,
    )
    .await?;

    let result = crate::domains::files::trash_object(
        &state.db,
        &access,
        object_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/objects/{objectId}/restore",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    responses(
        (status = 200, description = "Object restored", body = ObjectResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn restore_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ObjectResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;
    let resource = resource_context_for_object(
        &state.db,
        workspace_id,
        access.auth.user_id,
        access.auth.principal_id,
        object_id,
    )
    .await?;
    authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::RestoreObject,
        resource,
    )
    .await?;

    let result = crate::domains::files::restore_object(
        &state.db,
        &access,
        object_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}
