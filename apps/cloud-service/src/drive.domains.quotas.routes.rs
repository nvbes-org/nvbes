use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
};
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::error::AppError,
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new().route("/workspaces/{workspaceId}/quota", get(get_quota))
}

use crate::domains::quotas::QuotaResponse;

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/quota",
    tag = "quotas",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Workspace quota", body = QuotaResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn get_quota(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<QuotaResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewQuota,
        ResourceContext::default(),
    )
    .await?;

    let result = crate::domains::quotas::get_quota(&state.db, &access).await?;

    Ok(Json(result))
}
