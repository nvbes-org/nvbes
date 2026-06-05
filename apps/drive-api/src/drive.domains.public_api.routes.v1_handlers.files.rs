use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, Uri},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::{error::ErrorEnvelope, etag::if_match};

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction},
    domains::files::{CreateFolderInput, ListObjectsInput, MoveObjectInput, RenameObjectInput},
    http::error::AppError,
    http::request::{client_ip, user_agent},
};

use super::super::routes_helpers::*;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ListObjectsQuery {
    pub parent_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateFolderRequest {
    pub parent_id: Option<Uuid>,
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct RenameObjectRequest {
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct MoveObjectRequest {
    pub destination_parent_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspaceId}/objects",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("parentId" = Option<Uuid>, Query, description = "Parent folder ID"),
    ),
    responses(
        (status = 200, description = "List objects", body = crate::domains::files::ListObjectsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_objects(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListObjectsQuery>,
) -> Result<Json<crate::domains::files::ListObjectsResponse>, AppError> {
    let (ctx, access) = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:read",
        WorkspaceAction::ViewFiles,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::files::list_objects(
        &state.db,
        &access,
        ListObjectsInput {
            parent_id: query.parent_id,
        },
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "GET",
        "/v1/workspaces/:workspaceId/objects",
        &["files:read"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/folders",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreateFolderRequest,
    responses(
        (status = 200, description = "Folder created", body = crate::domains::files::ObjectResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn create_folder(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateFolderRequest>,
) -> Result<Json<crate::domains::files::ObjectResponse>, AppError> {
    let (ctx, access) = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:write",
        WorkspaceAction::CreateFolder,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::files::create_folder(
        &state.db,
        &access,
        CreateFolderInput {
            parent_id: request.parent_id,
            name: request.name,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "POST",
        "/v1/workspaces/:workspaceId/folders",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}

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
    let (ctx, access) = scoped_object_access(
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
        &access,
        object_id,
        RenameObjectInput { name: request.name },
        if_match(&headers)?,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
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
    let (ctx, access) = scoped_object_access(
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
        &access,
        object_id,
        MoveObjectInput {
            destination_parent_id: request.destination_parent_id,
        },
        if_match(&headers)?,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
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
    let (ctx, access) = scoped_object_access(
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
        &access,
        object_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    record_api_event(
        &state.db,
        &headers,
        &ctx,
        "api.file.trashed",
        "storage_object",
        Some(result.object.id),
        serde_json::json!({ "object_id": object_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "POST",
        "/v1/workspaces/:workspaceId/objects/:objectId/trash",
        &["files:delete"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/objects/{objectId}/download-url",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    responses(
        (status = 200, description = "Download URL created", body = crate::domains::files::DownloadUrlResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn create_download_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::domains::files::DownloadUrlResponse>, AppError> {
    let (ctx, access) = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:read",
        WorkspaceAction::DownloadFile,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::files::create_download_url(
        state.storage.as_ref(),
        &state.db,
        &access,
        object_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    record_api_event(
        &state.db,
        &headers,
        &ctx,
        "api.file.download_url_created",
        "storage_object",
        Some(object_id),
        serde_json::json!({ "object_id": object_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "POST",
        "/v1/workspaces/:workspaceId/objects/:objectId/download-url",
        &["files:read"],
    )
    .await?;
    Ok(Json(result))
}
