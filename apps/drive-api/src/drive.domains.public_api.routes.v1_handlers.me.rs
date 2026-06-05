use super::super::routes_helpers::*;
use super::super::types::*;
use crate::{app::AppState, http::error::AppError};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Method, Uri},
};
use nvbes_core::http::error::ErrorEnvelope;

#[utoipa::path(
    get,
    path = "/v1/me",
    tag = "public-api",
    responses(
        (status = 200, description = "Current API identity", body = ApiIdentityResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
) -> Result<Json<ApiIdentityResponse>, AppError> {
    let ctx = authenticate(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        "files:read",
    )
    .await?;
    let result = crate::domains::public_api::me(&ctx).await;
    log_ok(&state.db, &headers, &ctx, "GET", "/v1/me", &["files:read"]).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/v1/workspaces",
    tag = "public-api",
    responses(
        (status = 200, description = "List workspaces", body = PublicWorkspacesResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_workspaces(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
) -> Result<Json<PublicWorkspacesResponse>, AppError> {
    let ctx = authenticate(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        "files:read",
    )
    .await?;
    let result = crate::domains::public_api::list_workspaces(&state.db, &ctx).await?;
    log_ok(
        &state.db,
        &headers,
        &ctx,
        "GET",
        "/v1/workspaces",
        &["files:read"],
    )
    .await?;
    Ok(Json(result))
}
