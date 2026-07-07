use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use crate::domains::files::service::{
    CreateFolderInput, ListObjectsInput, ListObjectsResponse, ObjectResponse, TrashListResponse,
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/objects", get(list_objects))
        .route("/workspaces/{workspaceId}/folders", post(create_folder))
        .route("/workspaces/{workspaceId}/trash", get(list_trash))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
struct ListObjectsQuery {
    parent_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
struct CreateFolderRequest {
    parent_id: Option<Uuid>,
    name: String,
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/objects",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("parentId" = Option<Uuid>, Query, description = "Parent folder ID to list children of"),
    ),
    responses(
        (status = 200, description = "List objects in workspace", body = ListObjectsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn list_objects(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListObjectsQuery>,
) -> Result<Json<ListObjectsResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
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

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/folders",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreateFolderRequest,
    responses(
        (status = 200, description = "Folder created", body = ObjectResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn create_folder(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateFolderRequest>,
) -> Result<Json<ObjectResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
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

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/trash",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List trashed objects", body = TrashListResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn list_trash(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<TrashListResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewTrash,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::files::list_trash(&state.db, &access).await?;

    Ok(Json(result))
}
