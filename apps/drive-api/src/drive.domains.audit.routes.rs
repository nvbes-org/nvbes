use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::get,
};
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::error::AppError,
};

use super::service::{AuditEventsResponse, ListAuditEventsInput};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/audit-events",
            get(list_audit_events),
        )
        .route(
            "/workspaces/{workspaceId}/audit-events/export",
            get(export_audit_events),
        )
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/audit-events",
    tag = "audit",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("limit" = Option<i64>, Query, description = "Max events to return"),
        ("before" = Option<Uuid>, Query, description = "Cursor: return events before this ID"),
        ("action" = Option<String>, Query, description = "Filter by action"),
        ("actorUserId" = Option<Uuid>, Query, description = "Filter by actor user ID"),
    ),
    responses(
        (status = 200, description = "Audit events", body = AuditEventsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn list_audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListAuditEventsInput>,
) -> Result<Json<AuditEventsResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewAudit,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::audit::list_events(&state.db, &access, query).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/audit-events/export",
    tag = "audit",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Audit events CSV export", content_type = "text/csv"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn export_audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ExportAudit,
        ResourceContext::default(),
    )
    .await?;

    let export = crate::domains::audit::export_events(&state.db, &access).await?;
    Ok((
        [
            (header::CONTENT_TYPE, export.content_type.to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", export.filename),
            ),
        ],
        export.body,
    ))
}
