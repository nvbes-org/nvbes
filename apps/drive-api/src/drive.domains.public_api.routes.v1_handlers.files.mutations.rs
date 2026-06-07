use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, Method, Uri},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::WorkspaceAction,
    domains::files::{MoveObjectInput, RenameObjectInput},
    http::error::AppError,
};
use nvbes_core::http::{error::ErrorEnvelope, etag::if_match};

use super::super::super::{
    routes_access::{log_ok, scoped_object_access},
    routes_audit::record_api_event,
};
use super::{MoveObjectRequest, RenameObjectRequest};

#[utoipa::path(
    patch,
    path = "/v1/workspaces/{workspaceId}/objects/{objectId}",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    request_body = RenameObjectRequest,
    responses(
        (status = 200, description = "Object renamed", body = crate::domains::files::ObjectResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn rename_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RenameObjectRequest>,
) -> Result<Json<crate::domains::files::ObjectResponse>, AppError> {
    let authorized = scoped_object_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:write",
        object_id,
        WorkspaceAction::RenameObject,
    )
    .await?;
    let result = crate::domains::files::rename_object(
        &state.db,
        &authorized.access,
        object_id,
        RenameObjectInput { name: request.name },
        if_match(&headers)?,
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "PATCH",
        "/v1/workspaces/:workspaceId/objects/:objectId",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/objects/{objectId}/move",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    request_body = MoveObjectRequest,
    responses(
        (status = 200, description = "Object moved", body = crate::domains::files::ObjectResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn move_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<MoveObjectRequest>,
) -> Result<Json<crate::domains::files::ObjectResponse>, AppError> {
    let authorized = scoped_object_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:write",
        object_id,
        WorkspaceAction::MoveObject,
    )
    .await?;
    let result = crate::domains::files::move_object(
        &state.db,
        &authorized.access,
        object_id,
        MoveObjectInput {
            destination_parent_id: request.destination_parent_id,
        },
        if_match(&headers)?,
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/objects/:objectId/move",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/objects/{objectId}/trash",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    responses(
        (status = 200, description = "Object trashed", body = crate::domains::files::ObjectResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn trash_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::domains::files::ObjectResponse>, AppError> {
    let authorized = scoped_object_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:delete",
        object_id,
        WorkspaceAction::TrashObject,
    )
    .await?;
    let result = crate::domains::files::trash_object(
        &state.db,
        &authorized.access,
        object_id,
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    record_api_event(
        &state.db,
        &authorized.request,
        "api.file.trashed",
        "storage_object",
        Some(result.object.id),
        serde_json::json!({ "object_id": object_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/objects/:objectId/trash",
        &["files:delete"],
    )
    .await?;
    Ok(Json(result))
}
