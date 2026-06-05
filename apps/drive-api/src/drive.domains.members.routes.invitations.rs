use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::Deserialize;
use uuid::Uuid;

use nvbes_product_analytics::ProductAnalyticsEvent;

use super::service::{
    AcceptInvitationInput, AcceptInvitationResponse, InviteMemberInput, InviteMemberResponse,
};
use crate::{
    app::AppState,
    domains::authz::{
        ResourceContext, WorkspaceAction, authorize_workspace_action, ensure_email_verified,
        parse_role,
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/invitations", post(invite_member))
        .route("/invitations/accept", post(accept_invitation))
}

#[derive(Deserialize)]
struct InviteMemberRequest {
    email: String,
    role: String,
}

#[derive(Deserialize)]
struct AcceptInvitationRequest {
    token: String,
}

async fn invite_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<InviteMemberRequest>,
) -> Result<Json<InviteMemberResponse>, AppError> {
    let target_role = parse_role(&request.role)?;
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::InviteMember,
        ResourceContext {
            target_role: Some(target_role),
            ..ResourceContext::default()
        },
    )
    .await?;

    let result = crate::domains::members::invite_member(
        &state.db,
        &state.config,
        &access,
        InviteMemberInput {
            email: request.email,
            role: target_role,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    state.product_analytics.capture(
        ProductAnalyticsEvent::workspace_for_user(
            "member.invited",
            access.auth.user_id,
            workspace_id,
        )
        .property("member_count", 1_i64),
    );

    Ok(Json(result))
}

async fn accept_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AcceptInvitationRequest>,
) -> Result<Json<AcceptInvitationResponse>, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;
    ensure_email_verified(&auth)?;
    let result = crate::domains::members::accept_invitation(
        &state.db,
        &auth,
        AcceptInvitationInput {
            token: request.token,
        },
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;

    Ok(Json(result))
}
