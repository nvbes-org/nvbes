use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, Method, Uri},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction},
    domains::share_links::{CreateShareLinkInput, UpdateShareLinkInput},
    http::error::AppError,
    http::request::{client_ip, user_agent},
};

use super::super::routes_helpers::*;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateShareLinkRequest {
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpdateShareLinkRequest {
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<Option<i32>>,
}

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspaceId}/share-links",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List share links", body = crate::domains::share_links::ShareLinkListResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_share_links(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<crate::domains::share_links::ShareLinkListResponse>, AppError> {
    let (ctx, access) = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "share_links:read",
        WorkspaceAction::ViewShareLinks,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::share_links::list_share_links(&state.db, &access).await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "GET",
        "/v1/workspaces/:workspaceId/share-links",
        &["share_links:read"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/{workspaceId}/objects/{objectId}/share-links",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    request_body = CreateShareLinkRequest,
    responses(
        (status = 200, description = "Share link created", body = crate::domains::share_links::ShareLinkResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn create_share_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateShareLinkRequest>,
) -> Result<Json<crate::domains::share_links::ShareLinkResponse>, AppError> {
    let (ctx, access) = scoped_object_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "share_links:write",
        object_id,
        WorkspaceAction::CreateShareLink,
    )
    .await?;
    let result = crate::domains::share_links::create_share_link(
        &state.db,
        &access,
        object_id,
        CreateShareLinkInput {
            expires_at: request.expires_at,
            max_downloads: request.max_downloads,
        },
        crate::domains::share_links::logic::public_share_requires_clean_scan(
            &state.config.environment,
        ),
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    record_api_event(
        &state.db,
        &headers,
        &ctx,
        "api.share_link.created",
        "share_link",
        Some(result.share_link.id),
        serde_json::json!({ "storage_object_id": object_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "POST",
        "/v1/workspaces/:workspaceId/objects/:objectId/share-links",
        &["share_links:write"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    patch,
    path = "/v1/workspaces/{workspaceId}/share-links/{shareLinkId}",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("shareLinkId" = Uuid, Path, description = "Share link ID"),
    ),
    request_body = UpdateShareLinkRequest,
    responses(
        (status = 200, description = "Share link updated", body = crate::domains::share_links::ShareLinkResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn update_share_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, share_link_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateShareLinkRequest>,
) -> Result<Json<crate::domains::share_links::ShareLinkResponse>, AppError> {
    let (ctx, access) = scoped_share_link_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "share_links:write",
        share_link_id,
        WorkspaceAction::UpdateShareLink,
    )
    .await?;
    let result = crate::domains::share_links::update_share_link(
        &state.db,
        &access,
        share_link_id,
        UpdateShareLinkInput {
            expires_at: request.expires_at,
            max_downloads: request.max_downloads,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "PATCH",
        "/v1/workspaces/:workspaceId/share-links/:shareLinkId",
        &["share_links:write"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    delete,
    path = "/v1/workspaces/{workspaceId}/share-links/{shareLinkId}",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("shareLinkId" = Uuid, Path, description = "Share link ID"),
    ),
    responses(
        (status = 200, description = "Share link revoked", body = crate::domains::share_links::ShareLinkResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn revoke_share_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path((workspace_id, share_link_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::domains::share_links::ShareLinkResponse>, AppError> {
    let (ctx, access) = scoped_share_link_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "share_links:write",
        share_link_id,
        WorkspaceAction::RevokeShareLink,
    )
    .await?;
    let result = crate::domains::share_links::revoke_share_link(
        &state.db,
        &access,
        share_link_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    record_api_event(
        &state.db,
        &headers,
        &ctx,
        "api.share_link.revoked",
        "share_link",
        Some(result.share_link.id),
        serde_json::json!({ "share_link_id": share_link_id }),
    )
    .await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "DELETE",
        "/v1/workspaces/:workspaceId/share-links/:shareLinkId",
        &["share_links:write"],
    )
    .await?;
    Ok(Json(result))
}
