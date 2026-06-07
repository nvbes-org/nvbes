use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, Method, Uri},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction},
    http::error::AppError,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::super::super::{
    routes_access::{log_ok, scoped_access},
    routes_audit::record_api_event,
};

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
    let authorized = scoped_access(
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
        &authorized.access,
        object_id,
        authorized.request.meta.ip_owned(),
        authorized.request.meta.user_agent_owned(),
    )
    .await?;
    record_api_event(
        &state.db,
        &authorized.request,
        "api.file.download_url_created",
        "storage_object",
        Some(object_id),
        serde_json::json!({ "object_id": object_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &authorized.request,
        "POST",
        "/v1/workspaces/:workspaceId/objects/:objectId/download-url",
        &["files:read"],
    )
    .await?;
    Ok(Json(result))
}
