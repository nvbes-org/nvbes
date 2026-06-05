use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::authz::{
    ResourceContext, WorkspaceAction, authorize_workspace_action, ensure_email_verified, parse_role,
};
use crate::domains::members::service::{
    self, AcceptInvitationInput, AcceptInvitationResponse, InviteMemberInput, InviteMemberResponse,
};
use crate::http::error::AppError;
use crate::http::request::{bearer_token, client_ip, user_agent};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/workspaces/{workspaceId}/members", post(invite_member))
        .route("/invitations/accept", post(accept_invitation))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct InviteMemberRequest {
    email: String,
    role: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct AcceptInvitationRequest {
    token: String,
}

#[utoipa::path(
    post,
    path = "/workspaces/{workspaceId}/members",
    tag = "members",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    request_body = InviteMemberRequest,
    responses(
        (status = 200, description = "Member invited", body = InviteMemberResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn invite_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<InviteMemberRequest>,
) -> Result<Json<InviteMemberResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::InviteMember,
        ResourceContext::default(),
    )
    .await?;

    Ok(Json(
        service::invite_member(
            &state.db,
            &state.redis,
            &state.config,
            &access,
            InviteMemberInput {
                email: request.email,
                role: parse_role(&request.role)?,
            },
            client_ip(&headers),
            user_agent(&headers),
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/invitations/accept",
    tag = "members",
    request_body = AcceptInvitationRequest,
    responses(
        (status = 200, description = "Invitation accepted", body = AcceptInvitationResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn accept_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AcceptInvitationRequest>,
) -> Result<Json<AcceptInvitationResponse>, AppError> {
    let token = bearer_token(&headers)?;
    let auth = sessions::authenticate(&state.db, &state.redis, &state.jwt, &token).await?;
    ensure_email_verified(&auth)?;

    Ok(Json(
        service::accept_invitation(
            &state.db,
            &auth,
            AcceptInvitationInput {
                token: request.token,
            },
            client_ip(&headers),
            user_agent(&headers),
        )
        .await?,
    ))
}
