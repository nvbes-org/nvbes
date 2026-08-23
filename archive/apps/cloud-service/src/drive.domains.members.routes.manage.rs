use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, patch},
};
use serde::Deserialize;
use uuid::Uuid;

use super::service::{
    ListMembersInput, MemberListResponse, RemoveMemberResponse, UpdateMemberInput,
    UpdateMemberResponse,
};
use crate::{
    app::AppState,
    domains::authz::{
        ResourceContext, WorkspaceAction, authorize_workspace_action, parse_role,
        target_role_for_member,
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/members", get(list_members))
        .route(
            "/workspaces/{workspaceId}/members/{memberId}",
            patch(update_member).delete(remove_member),
        )
}

#[derive(Deserialize)]
struct UpdateMemberRequest {
    role: String,
}

#[derive(Deserialize)]
struct ListMembersQuery {
    limit: Option<i64>,
    cursor: Option<String>,
    role: Option<String>,
    email_prefix: Option<String>,
}

async fn list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListMembersQuery>,
) -> Result<Json<MemberListResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ViewMembers,
        ResourceContext::default(),
    )
    .await?;

    let role = query.role.as_deref().map(parse_role).transpose()?;
    let result = crate::domains::members::list_members(
        &state.db,
        &access,
        ListMembersInput {
            limit: query.limit,
            cursor: query.cursor,
            role,
            email_prefix: query.email_prefix,
        },
    )
    .await?;

    Ok(Json(result))
}

async fn update_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, member_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateMemberRequest>,
) -> Result<Json<UpdateMemberResponse>, AppError> {
    let _current_role = target_role_for_member(&state.db, workspace_id, member_id).await?;
    let target_role = parse_role(&request.role)?;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::ChangeMemberRole,
        ResourceContext {
            target_role: Some(target_role),
            ..ResourceContext::default()
        },
    )
    .await?;

    let result = crate::domains::members::update_member_role(
        &state.db,
        &state.redis,
        &access,
        member_id,
        UpdateMemberInput { role: target_role },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}

async fn remove_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RemoveMemberResponse>, AppError> {
    let target_role = target_role_for_member(&state.db, workspace_id, member_id).await?;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::RemoveMember,
        ResourceContext {
            target_role: Some(target_role),
            ..ResourceContext::default()
        },
    )
    .await?;

    let result = crate::domains::members::remove_member(
        &state.db,
        &state.redis,
        &access,
        member_id,
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}
