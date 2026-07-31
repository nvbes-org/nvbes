use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
};
use nvbes_core::http::error::ErrorEnvelope;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::error::AppError,
};

use super::service;
use super::types::{ListSecurityEventsInput, SecurityEventsResponse};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/security-events",
            get(list_security_events),
        )
        .route(
            "/workspaces/{workspaceId}/security-events/export",
            get(export_security_events),
        )
}

fn security_events_action() -> WorkspaceAction {
    WorkspaceAction::ExportAudit
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/security-events",
    tag = "security",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("limit" = Option<i64>, Query, description = "Max results"),
        ("cursor" = Option<String>, Query, description = "Opaque pagination cursor"),
        ("geo_country_code" = Option<String>, Query, description = "Filter by resolved ISO country code"),
        ("geo_source" = Option<String>, Query, description = "Filter by geo source"),
        ("geo_confidence" = Option<String>, Query, description = "Filter by geo confidence"),
        ("geo_network_kind" = Option<String>, Query, description = "Filter by geo network kind"),
        ("min_geo_risk_score" = Option<i64>, Query, description = "Minimum geo network risk score"),
        ("geo_risk_label" = Option<String>, Query, description = "Filter by exact geo risk label"),
    ),
    responses(
        (status = 200, description = "Security events", body = SecurityEventsResponse),
        (status = 400, description = "Invalid pagination cursor", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_security_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListSecurityEventsInput>,
) -> Result<Json<SecurityEventsResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        security_events_action(),
        ResourceContext::default(),
    )
    .await?;

    let result = service::list_events(&state.db, &access, query).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/security-events/export",
    tag = "security",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Security events export file"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn export_security_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        security_events_action(),
        ResourceContext::default(),
    )
    .await?;

    let export = service::export_events(&state.db, &access).await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                export.content_type.to_owned(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", export.filename),
            ),
        ],
        export.body,
    ))
}

#[cfg(test)]
mod tests {
    use super::security_events_action;
    use crate::domains::authz::WorkspaceAction;

    #[test]
    fn security_routes_use_export_audit_action() {
        assert!(matches!(
            security_events_action(),
            WorkspaceAction::ExportAudit
        ));
    }
}
