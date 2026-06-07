use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    domains::uploads::service::{CreateUploadInput, CreateUploadResponse},
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};
use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::{ProductAnalyticsEvent, size_bytes_bucket};

use super::types::CreateUploadRequest;

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/uploads",
    tag = "uploads",
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
pub(super) async fn create_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    body: Bytes,
) -> Result<Response, AppError> {
    if super::super::tus::is_tus_request(&headers) {
        return super::super::tus::create_tus_upload(State(state), headers, Path(workspace_id))
            .await;
    }

    let request: CreateUploadRequest = serde_json::from_slice(&body).map_err(|error| {
        AppError::bad_request(
            "invalid_json",
            format!("Invalid upload request body: {error}"),
        )
    })?;

    let started_at = Instant::now();
    let expected_size_bytes = request.expected_size_bytes;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::uploads::create_upload(
        state.storage.as_ref(),
        &state.db,
        &access,
        CreateUploadInput {
            parent_id: request.parent_id,
            name: request.name,
            mime_type: request.mime_type,
            expected_size_bytes: request.expected_size_bytes,
            expected_checksum: request.expected_checksum,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await;

    match &result {
        Ok(_) => state.observability.record_upload_operation(
            "create",
            "success",
            Some(expected_size_bytes as u64),
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_upload_operation(
            "create",
            "failure",
            None,
            started_at.elapsed(),
        ),
    }

    if result.is_ok() {
        state.product_analytics.capture(
            ProductAnalyticsEvent::workspace_for_user(
                "file.upload_started",
                access.auth.user_id,
                workspace_id,
            )
            .property("file_count", 1_i64)
            .property("upload_count", 1_i64)
            .property("size_bytes_bucket", size_bytes_bucket(expected_size_bytes)),
        );
    }

    Ok(Json(result?).into_response())
}
