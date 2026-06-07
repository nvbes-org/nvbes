use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, Method, Uri},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction},
    domains::uploads::types::{
        CancelUploadResponse, CompleteUploadInput, CompleteUploadResponse, CreateUploadInput,
        CreateUploadResponse,
    },
    http::error::AppError,
};

use super::super::{
    routes_access::{log_ok, scoped_access},
    routes_audit::record_api_event,
};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateUploadRequest {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub mime_type: String,
    pub expected_size_bytes: i64,
    pub expected_checksum: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CompleteUploadRequest {
    pub size_bytes: i64,
    pub checksum: Option<String>,
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/uploads",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = CreateUploadRequest,
    responses(
        (status = 200, description = "Upload created", body = CreateUploadResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn create_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateUploadRequest>,
) -> Result<Json<CreateUploadResponse>, AppError> {
    let authorized = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:write",
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::uploads::create_upload(
        state.storage.as_ref(),
        &state.db,
        &authorized.access,
        CreateUploadInput {
            parent_id: request.parent_id,
            name: request.name,
            mime_type: request.mime_type,
            expected_size_bytes: request.expected_size_bytes,
            expected_checksum: request.expected_checksum,
        },
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/uploads",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("uploadId" = Uuid, Path, description = "Upload ID"),
    ),
    request_body = CompleteUploadRequest,
    responses(
        (status = 200, description = "Upload completed", body = CompleteUploadResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn complete_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, upload_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CompleteUploadRequest>,
) -> Result<Json<CompleteUploadResponse>, AppError> {
    let authorized = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:write",
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::uploads::complete_upload(
        state.storage.as_ref(),
        state.scanner.as_ref(),
        &state.db,
        &authorized.access,
        upload_id,
        CompleteUploadInput {
            size_bytes: request.size_bytes,
            checksum: request.checksum,
        },
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
        state.config.scan_enabled,
        state.config.scan_fail_open,
        &state.config.scan_engine,
    )
    .await?;
    record_api_event(
        &state.db,
        &authorized.request,
        "api.file.uploaded",
        "storage_object",
        Some(result.storage_object.id),
        serde_json::json!({ "upload_id": upload_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/uploads/:uploadId/complete",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/uploads/{uploadId}/cancel",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("uploadId" = Uuid, Path, description = "Upload ID"),
    ),
    responses(
        (status = 200, description = "Upload cancelled", body = CancelUploadResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn cancel_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, upload_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CancelUploadResponse>, AppError> {
    let authorized = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "files:write",
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::uploads::cancel_upload(
        state.storage.as_ref(),
        &state.db,
        &authorized.access,
        upload_id,
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/uploads/:uploadId/cancel",
        &["files:write"],
    )
    .await?;
    Ok(Json(result))
}
