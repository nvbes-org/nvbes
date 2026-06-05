use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    middleware,
    response::{IntoResponse, Response},
    routing::{head, options, post},
};
use serde::Deserialize;
use std::time::Instant;
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

use super::service::*;
use super::tus;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/uploads", post(create_upload))
        .route(
            "/workspaces/{workspaceId}/uploads",
            options(tus::tus_options),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}",
            head(tus::tus_upload_head).patch(tus::tus_upload_patch),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}",
            options(tus::tus_options),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}/complete",
            post(complete_upload),
        )
        .route(
            "/workspaces/{workspaceId}/uploads/{uploadId}/cancel",
            post(cancel_upload),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::http::expect_continue::authenticate_before_body,
        ))
}

#[derive(Deserialize, ToSchema)]
struct CreateUploadRequest {
    #[serde(alias = "parentId")]
    parent_id: Option<Uuid>,
    name: String,
    #[serde(alias = "mimeType")]
    mime_type: String,
    #[serde(alias = "expectedSizeBytes")]
    expected_size_bytes: i64,
    #[serde(alias = "expectedChecksum")]
    expected_checksum: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct CompleteUploadRequest {
    #[serde(alias = "sizeBytes")]
    size_bytes: i64,
    checksum: Option<String>,
}

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
async fn create_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    body: Bytes,
) -> Result<Response, AppError> {
    if tus::is_tus_request(&headers) {
        return tus::create_tus_upload(State(state), headers, Path(workspace_id)).await;
    }

    let request: CreateUploadRequest = serde_json::from_slice(&body).map_err(|e| {
        AppError::bad_request("invalid_json", format!("Invalid upload request body: {e}"))
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

    Ok(Json(result?).into_response())
}

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
async fn complete_upload(
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
async fn cancel_upload(
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
