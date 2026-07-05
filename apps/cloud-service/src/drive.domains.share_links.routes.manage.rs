use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch, post},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;
use nvbes_product_analytics::ProductAnalyticsEvent;

use crate::{
    app::AppState,
    domains::authz::{
        ResourceContext, WorkspaceAction, authorize_workspace_action, resource_context_for_object,
        resource_context_for_share_link,
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use super::{CreateShareLinkInput, ShareLinkListResponse, ShareLinkResponse, UpdateShareLinkInput};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/share-links",
            get(list_share_links),
        )
        .route(
            "/workspaces/{workspaceId}/objects/{objectId}/share-links",
            post(create_share_link),
        )
        .route(
            "/workspaces/{workspaceId}/share-links/{shareLinkId}",
            patch(update_share_link).delete(revoke_share_link),
        )
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/share-links",
    tag = "share-links",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List share links", body = ShareLinkListResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn list_share_links(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<ShareLinkListResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewShareLinks,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::share_links::list_share_links(&state.db, &access).await?;
    Ok(Json(result))
}

#[derive(Deserialize, ToSchema)]
struct CreateShareLinkRequest {
    expires_at: Option<DateTime<Utc>>,
    max_downloads: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
struct UpdateShareLinkRequest {
    expires_at: Option<DateTime<Utc>>,
    max_downloads: Option<Option<i32>>,
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/objects/{objectId}/share-links",
    tag = "share-links",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("objectId" = Uuid, Path, description = "Object ID"),
    ),
    request_body = CreateShareLinkRequest,
    responses(
        (status = 200, description = "Share link created", body = ShareLinkResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn create_share_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, object_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateShareLinkRequest>,
) -> Result<Json<ShareLinkResponse>, AppError> {
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
        WorkspaceAction::CreateShareLink,
        resource,
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

    state.product_analytics.capture(
        ProductAnalyticsEvent::workspace_for_user(
            "share_link.created",
            access.auth.user_id,
            result.share_link.workspace_id,
        )
        .property("share_link_count", 1_i64),
    );

    Ok(Json(result))
}

#[utoipa::path(
    patch,
    path = "/workspaces/{workspaceId}/share-links/{shareLinkId}",
    tag = "share-links",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("shareLinkId" = Uuid, Path, description = "Share link ID"),
    ),
    request_body = UpdateShareLinkRequest,
    responses(
        (status = 200, description = "Share link updated", body = ShareLinkResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn update_share_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, share_link_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateShareLinkRequest>,
) -> Result<Json<ShareLinkResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;
    let resource = resource_context_for_share_link(
        &state.db,
        workspace_id,
        access.auth.user_id,
        access.auth.principal_id,
        share_link_id,
    )
    .await?;
    authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateShareLink,
        resource,
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

    Ok(Json(result))
}

#[utoipa::path(
    delete,
    path = "/workspaces/{workspaceId}/share-links/{shareLinkId}",
    tag = "share-links",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("shareLinkId" = Uuid, Path, description = "Share link ID"),
    ),
    responses(
        (status = 200, description = "Share link revoked", body = ShareLinkResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn revoke_share_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, share_link_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ShareLinkResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewWorkspace,
        ResourceContext::default(),
    )
    .await?;
    let resource = resource_context_for_share_link(
        &state.db,
        workspace_id,
        access.auth.user_id,
        access.auth.principal_id,
        share_link_id,
    )
    .await?;
    authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::RevokeShareLink,
        resource,
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

    Ok(Json(result))
}
