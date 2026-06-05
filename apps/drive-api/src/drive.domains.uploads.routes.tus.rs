use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use super::super::logic;
use super::service::*;

#[path = "drive.domains.uploads.routes.tus.headers.rs"]
mod tus_headers;

use tus_headers::*;

pub(super) fn is_tus_request(headers: &HeaderMap) -> bool {
    headers.contains_key(TUS_RESUMABLE)
}

pub(super) async fn create_tus_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Response, AppError> {
    ensure_tus_version(&headers)?;
    ensure_supported_content_encoding(&headers)?;
    let upload_length = required_i64_header(&headers, UPLOAD_LENGTH)?;
    let metadata = parse_tus_metadata(&headers)?;
    let metadata_content_encoding = metadata
        .get("content_encoding")
        .or_else(|| metadata.get("content-encoding"))
        .map(String::as_str);
    logic::ensure_plain_content_encoding(metadata_content_encoding)?;
    let name = metadata
        .get("filename")
        .or_else(|| metadata.get("name"))
        .cloned()
        .ok_or_else(|| {
            AppError::bad_request(
                "upload_metadata_missing_name",
                "Upload-Metadata must include filename or name.",
            )
        })?;
    let mime_type = metadata
        .get("filetype")
        .or_else(|| metadata.get("mime_type"))
        .cloned()
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let expected_checksum = metadata.get("checksum").cloned();
    let parent_id = metadata
        .get("parent_id")
        .map(|value| Uuid::parse_str(value))
        .transpose()
        .map_err(|e| {
            AppError::bad_request("invalid_parent_id", format!("Invalid parent_id: {e}"))
        })?;

    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::uploads::create_tus_upload(
        state.storage.as_ref(),
        &state.db,
        &access,
        CreateTusUploadInput {
            parent_id,
            name,
            mime_type,
            upload_length,
            expected_checksum,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    let mut response = StatusCode::CREATED.into_response();
    insert_header(response.headers_mut(), TUS_RESUMABLE, TUS_VERSION)?;
    insert_header(
        response.headers_mut(),
        header::LOCATION.as_str(),
        &result.tus_url,
    )?;
    insert_header(response.headers_mut(), UPLOAD_OFFSET, "0")?;
    Ok(response)
}

pub(super) async fn tus_upload_head(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, upload_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, AppError> {
    ensure_tus_version(&headers)?;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;

    let status =
        crate::domains::uploads::get_tus_upload_status(&state.db, &access, upload_id).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    insert_header(response.headers_mut(), TUS_RESUMABLE, TUS_VERSION)?;
    insert_header(
        response.headers_mut(),
        UPLOAD_OFFSET,
        &status.upload_offset_bytes.to_string(),
    )?;
    insert_header(
        response.headers_mut(),
        UPLOAD_LENGTH,
        &status.upload_length_bytes.to_string(),
    )?;
    Ok(response)
}

pub(super) async fn tus_upload_patch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, upload_id)): Path<(Uuid, Uuid)>,
    body: Bytes,
) -> Result<Response, AppError> {
    ensure_tus_version(&headers)?;
    ensure_tus_patch_content_type(&headers)?;
    ensure_supported_content_encoding(&headers)?;
    let upload_offset_bytes = required_i64_header(&headers, UPLOAD_OFFSET)?;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UploadFile,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::uploads::append_tus_chunk(
        state.storage.as_ref(),
        state.scanner.as_ref(),
        &state.db,
        &access,
        upload_id,
        AppendTusChunkInput {
            upload_offset_bytes,
            body: body.to_vec(),
        },
        client_ip(&headers),
        user_agent(&headers),
        state.config.scan_enabled,
        state.config.scan_fail_open,
        &state.config.scan_engine,
    )
    .await?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    insert_header(response.headers_mut(), TUS_RESUMABLE, TUS_VERSION)?;
    insert_header(
        response.headers_mut(),
        UPLOAD_OFFSET,
        &result.upload_offset_bytes.to_string(),
    )?;
    Ok(response)
}

pub(super) async fn tus_options() -> Result<Response, AppError> {
    let mut response = StatusCode::NO_CONTENT.into_response();
    insert_header(response.headers_mut(), TUS_RESUMABLE, TUS_VERSION)?;
    insert_header(response.headers_mut(), TUS_VERSION_HEADER, TUS_VERSION)?;
    insert_header(response.headers_mut(), TUS_EXTENSION, "creation")?;
    insert_header(
        response.headers_mut(),
        TUS_MAX_SIZE,
        &logic::max_upload_bytes().to_string(),
    )?;
    Ok(response)
}
