use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use std::time::Instant;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    domains::uploads::service::{
        CancelUploadResponse, CompleteUploadInput, CompleteUploadResponse,
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};
use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::{ProductAnalyticsEvent, size_bytes_bucket};

use super::types::CompleteUploadRequest;

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/uploads/{uploadId}/complete",
    tag = "uploads",
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
pub(super) async fn complete_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, upload_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CompleteUploadRequest>,
) -> Result<Json<CompleteUploadResponse>, AppError> {
    let started_at = Instant::now();
    let size_bytes = request.size_bytes;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::uploads::complete_upload(
        state.storage.as_ref(),
        state.scanner.as_ref(),
        &state.db,
        &access,
        upload_id,
        CompleteUploadInput {
            size_bytes: request.size_bytes,
            checksum: request.checksum,
        },
        client_ip(&headers),
        user_agent(&headers),
        state.config.scan_enabled,
        state.config.scan_fail_open,
        &state.config.scan_engine,
    )
    .await;

    match &result {
        Ok(_) => state.observability.record_upload_operation(
            "complete",
            "success",
            Some(size_bytes as u64),
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_upload_operation(
            "complete",
            "failure",
            None,
            started_at.elapsed(),
        ),
    }

    if result.is_ok() {
        state.product_analytics.capture(
            ProductAnalyticsEvent::workspace_for_user(
                "file.upload_completed",
                access.auth.user_id,
                workspace_id,
            )
            .property("file_count", 1_i64)
            .property("upload_count", 1_i64)
            .property("size_bytes_bucket", size_bytes_bucket(size_bytes)),
        );
    }

    Ok(Json(result?))
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/uploads/{uploadId}/cancel",
    tag = "uploads",
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
pub(super) async fn cancel_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, upload_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CancelUploadResponse>, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::uploads::cancel_upload(
        state.storage.as_ref(),
        &state.db,
        &access,
        upload_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await;

    match &result {
        Ok(_) => state.observability.record_upload_operation(
            "cancel",
            "success",
            None,
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_upload_operation(
            "cancel",
            "failure",
            None,
            started_at.elapsed(),
        ),
    }

    Ok(Json(result?))
}
