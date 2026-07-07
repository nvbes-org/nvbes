use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ResourceContext, WorkspaceDecision, decide_workspace_action, parse_action, parse_role,
};
use crate::app::AppState;
use crate::http::error::AppError;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/authz/decision", post(decide))
        .layer(axum::middleware::from_fn(
            nvbes_core::security::no_cache_headers,
        ))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub(crate) struct DecisionRequest {
    workspace_id: Uuid,
    action: String,
    resource: Option<DecisionResourceRequest>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub(crate) struct DecisionResourceRequest {
    #[serde(default)]
    owns_resource: bool,
    #[serde(default)]
    member_share_links_enabled: bool,
    target_role: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub(crate) struct DecisionResponse {
    decision: WorkspaceDecision,
}

#[utoipa::path(
    post,
    path = "/authz/decision",
    tag = "authz",
    request_body = DecisionRequest,
    responses(
        (status = 200, description = "Authorization decision", body = DecisionResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn decide(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DecisionRequest>,
) -> Result<Json<DecisionResponse>, AppError> {
    let action = parse_action(&request.action)?;
    let target_role = request
        .resource
        .as_ref()
        .and_then(|resource| resource.target_role.as_deref())
        .map(parse_role)
        .transpose()?;
    let resource = request.resource.unwrap_or(DecisionResourceRequest {
        owns_resource: false,
        member_share_links_enabled: false,
        target_role: None,
    });
    let decision = decide_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        request.workspace_id,
        action,
        ResourceContext {
            owns_resource: resource.owns_resource,
            member_share_links_enabled: resource.member_share_links_enabled,
            target_role,
        },
    )
    .await?;

    Ok(Json(DecisionResponse { decision }))
}
