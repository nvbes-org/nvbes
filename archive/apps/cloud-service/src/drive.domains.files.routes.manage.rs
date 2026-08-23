use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{patch, post},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;
use nvbes_core::http::etag::{if_match, insert_etag, resource_etag};

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

use crate::domains::files::service::{
    DeleteObjectResponse, MoveObjectInput, ObjectResponse, RenameObjectInput,
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}",
            patch(rename_object).delete(delete_object),
        )
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}/move",
            post(move_object),
        )
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
struct RenameObjectRequest {
    name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
struct MoveObjectRequest {
    destination_parent_id: Option<Uuid>,
}

#[utoipa::path(
    patch,
    path = "/workspaces/{workspaceId}/objects/{objectId}",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    request_body = RenameObjectRequest,
    responses(
        (status = 200, description = "Object renamed", body = ObjectResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn rename_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RenameObjectRequest>,
) -> Result<Response, AppError> {
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
        WorkspaceAction::RenameObject,
        resource,
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

    object_response(result)
}

#[utoipa::path(
    delete,
    path = "/workspaces/{workspaceId}/objects/{objectId}",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    responses(
        (status = 200, description = "Object deleted", body = DeleteObjectResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn delete_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteObjectResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::DeleteObjectPermanently,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::files::delete_object(
        &state.db,
        &access,
        object_id,
        if_match(&headers)?,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/objects/{objectId}/move",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    request_body = MoveObjectRequest,
    responses(
        (status = 200, description = "Object moved", body = ObjectResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn move_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<MoveObjectRequest>,
) -> Result<Response, AppError> {
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
        WorkspaceAction::MoveObject,
        resource,
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

    object_response(result)
}

fn object_response(result: ObjectResponse) -> Result<Response, AppError> {
    let etag = resource_etag("storage_object", result.object.id, result.object.updated_at);
    let mut headers = HeaderMap::new();
    insert_etag(&mut headers, &etag)?;
    Ok((headers, Json(result)).into_response())
}
