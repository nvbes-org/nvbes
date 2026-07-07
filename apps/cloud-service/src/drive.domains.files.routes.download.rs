use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::time::Instant;
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
    DownloadObjectInput, DownloadObjectResponse, DownloadObjectStatus, DownloadUrlResponse,
};

const ACCEPT_RANGES_VALUE: &str = "bytes";

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}/download",
            get(download_object),
        )
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}/download-url",
            post(create_download_url),
        )
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/objects/{objectId}/download-url",
    tag = "files",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    responses(
        (status = 200, description = "Download URL created", body = DownloadUrlResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn create_download_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DownloadUrlResponse>, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
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
    .await;

    match &result {
        Ok(_) => state.observability.record_download_operation(
            "create_url",
            "success",
            None,
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_download_operation(
            "create_url",
            "failure",
            None,
            started_at.elapsed(),
        ),
    }

    Ok(Json(result?))
}

async fn download_object(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, AppError> {
    let started_at = Instant::now();
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::DownloadFile,
        ResourceContext::default(),
    )
    .await?;

    let range_header = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let result = crate::domains::files::download_object(
        state.storage.as_ref(),
        &state.db,
        &access,
        object_id,
        DownloadObjectInput { range_header },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await;

    match &result {
        Ok(download) => state.observability.record_download_operation(
            "stream",
            "success",
            Some(download.served_bytes as u64),
            started_at.elapsed(),
        ),
        Err(_) => state.observability.record_download_operation(
            "stream",
            "failure",
            None,
            started_at.elapsed(),
        ),
    }

    build_download_response(result?)
}

fn build_download_response(download: DownloadObjectResponse) -> Result<Response, AppError> {
    let status = match download.status {
        DownloadObjectStatus::Full => StatusCode::OK,
        DownloadObjectStatus::Partial { .. } => StatusCode::PARTIAL_CONTENT,
        DownloadObjectStatus::Unsatisfiable => StatusCode::RANGE_NOT_SATISFIABLE,
    };

    let mut response = (status, Body::from(download.body)).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::ACCEPT_RANGES,
        HeaderValue::from_static(ACCEPT_RANGES_VALUE),
    );
    insert_header(
        headers,
        header::CONTENT_TYPE.as_str(),
        &download.content_type,
    )?;

    match download.status {
        DownloadObjectStatus::Full => {
            insert_header(
                headers,
                header::CONTENT_LENGTH.as_str(),
                &download.served_bytes.to_string(),
            )?;
        }
        DownloadObjectStatus::Partial {
            start,
            end_inclusive,
        } => {
            insert_header(
                headers,
                header::CONTENT_RANGE.as_str(),
                &format!("bytes {start}-{end_inclusive}/{}", download.size_bytes),
            )?;
            insert_header(
                headers,
                header::CONTENT_LENGTH.as_str(),
                &download.served_bytes.to_string(),
            )?;
        }
        DownloadObjectStatus::Unsatisfiable => {
            insert_header(
                headers,
                header::CONTENT_RANGE.as_str(),
                &format!("bytes */{}", download.size_bytes),
            )?;
            insert_header(headers, header::CONTENT_LENGTH.as_str(), "0")?;
        }
    }

    Ok(response)
}

fn insert_header(headers: &mut HeaderMap, name: &str, value: &str) -> Result<(), AppError> {
    let name = axum::http::HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
        AppError::internal(
            "invalid_response_header",
            format!("Invalid header name: {e}"),
        )
    })?;
    let value = HeaderValue::from_str(value).map_err(|e| {
        AppError::internal(
            "invalid_response_header",
            format!("Invalid header value: {e}"),
        )
    })?;
    headers.insert(name, value);
    Ok(())
}
