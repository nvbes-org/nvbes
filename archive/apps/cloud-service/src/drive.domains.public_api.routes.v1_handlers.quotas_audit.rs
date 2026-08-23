use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, Uri},
};
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::{
        audit::ListAuditEventsInput,
        authz::{ResourceContext, WorkspaceAction},
    },
    http::error::AppError,
};

use super::super::routes_access::{log_ok, scoped_access};

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspaceId}/quota",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Workspace quota", body = crate::domains::quotas::QuotaResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn get_quota(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<crate::domains::quotas::QuotaResponse>, AppError> {
    let authorized = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "quota:read",
        WorkspaceAction::ViewQuota,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::quotas::get_quota(&state.db, &authorized.access).await?;
    log_ok(
        &state.db,
        &authorized.request,
        "GET",
        "/v1/workspaces/:workspaceId/quota",
        &["quota:read"],
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspaceId}/audit-events",
    tag = "public-api",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("limit" = Option<i64>, Query, description = "Max events to return"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
        ("action" = Option<String>, Query, description = "Filter by action"),
        ("actorUserId" = Option<Uuid>, Query, description = "Filter by actor user ID"),
    ),
    responses(
        (status = 200, description = "Audit events", body = crate::domains::audit::AuditEventsResponse),
        (status = 400, description = "Invalid pagination cursor", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListAuditEventsInput>,
) -> Result<Json<crate::domains::audit::AuditEventsResponse>, AppError> {
    let authorized = scoped_access(
        &state.db,
        &state.redis,
        &headers,
        &method,
        &uri,
        workspace_id,
        "audit:read",
        WorkspaceAction::ViewAudit,
        ResourceContext::default(),
    )
    .await?;
    let result = crate::domains::audit::list_events(&state.db, &authorized.access, query).await?;
    log_ok(
        &state.db,
        &authorized.request,
        "GET",
        "/v1/workspaces/:workspaceId/audit-events",
        &["audit:read"],
    )
    .await?;
    Ok(Json(result))
}
