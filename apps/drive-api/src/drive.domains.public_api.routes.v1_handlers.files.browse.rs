use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, Uri},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction},
    domains::files::{CreateFolderInput, ListObjectsInput},
    http::error::AppError,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::super::super::routes_access::{log_ok, scoped_access};
use super::{CreateFolderRequest, ListObjectsQuery};

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
    let authorized = scoped_access(
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
        &authorized.access,
        ListObjectsInput {
            parent_id: query.parent_id,
        },
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
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
    let authorized = scoped_access(
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
        &authorized.access,
        CreateFolderInput {
            parent_id: request.parent_id,
            name: request.name,
        },
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/folders",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}
