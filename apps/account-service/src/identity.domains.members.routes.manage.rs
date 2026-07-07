use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::authz::{
    ResourceContext, WorkspaceAction, authorize_workspace_action, parse_role,
    target_role_for_member,
};
use crate::domains::members::service::{
    self, MemberListResponse, UpdateMemberInput, UpdateMemberResponse,
};
use crate::http::error::AppError;
use crate::http::request::{client_ip, user_agent};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/members", get(list_members))
        .route(
            "/workspaces/{workspaceId}/members/{memberId}",
            patch(update_member).delete(remove_member),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct UpdateMemberRequest {
    role: String,
}

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/members",
    tag = "members",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List members", body = MemberListResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<MemberListResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewMembers,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(service::list_members(&state.db, &access).await?))
}

#[utoipa::path(
    patch,
    path = "/workspaces/{workspaceId}/members/{memberId}",
    tag = "members",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("memberId" = Uuid, Path, description = "Member ID"),
    ),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Member updated", body = UpdateMemberResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn update_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, member_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateMemberRequest>,
) -> Result<Json<UpdateMemberResponse>, AppError> {
    let target_role = match target_role_for_member(&state.db, workspace_id, member_id).await {
        Ok(role) => Some(role),
        Err(error) if error.code == "member_not_found" => None,
        Err(error) => return Err(error),
    };
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ChangeMemberRole,
        ResourceContext {
            target_role,
            ..ResourceContext::default()
        },
    )
    .await?;

    Ok(Json(
        service::update_member_role(
            &state.db,
            &state.redis,
            &access,
            member_id,
            UpdateMemberInput {
                role: parse_role(&request.role)?,
            },
            client_ip(&headers),
            user_agent(&headers),
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/workspaces/{workspaceId}/members/{memberId}",
    tag = "members",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("memberId" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Member removed", body = crate::domains::members::service::RemoveMemberResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn remove_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::domains::members::service::RemoveMemberResponse>, AppError> {
    let target_role = match target_role_for_member(&state.db, workspace_id, member_id).await {
        Ok(role) => Some(role),
        Err(error) if error.code == "member_not_found" => None,
        Err(error) => return Err(error),
    };
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::RemoveMember,
        ResourceContext {
            target_role,
            ..ResourceContext::default()
        },
    )
    .await?;

    Ok(Json(
        service::remove_member(
            &state.db,
            &state.redis,
            &access,
            member_id,
            client_ip(&headers),
            user_agent(&headers),
        )
        .await?,
    ))
}
