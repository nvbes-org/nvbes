use crate::app::AppState;
use crate::domains::auth::sessions_context;
use crate::domains::auth::types::{SwitchWorkspaceInput, SwitchWorkspaceResult};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub(crate) struct SwitchWorkspaceRequest {
    pub(crate) password: Option<String>,
    pub(crate) totp_code: Option<String>,
    #[schema(value_type = Object)]
    pub(crate) webauthn_response: Option<webauthn_rs::prelude::PublicKeyCredential>,
    pub(crate) webauthn_challenge_id: Option<Uuid>,
    pub(crate) recovery_code: Option<String>,
}

#[utoipa::path(
    post,
    path = "/auth/workspaces/{workspaceId}/switch",
    tag = "auth",
    request_body = SwitchWorkspaceRequest,
    responses(
        (status = 200, description = "Workspace switched", body = SwitchWorkspaceResult),
        (status = 401, description = "Unauthorized or invalid credentials", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Workspace not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn switch_workspace(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<SwitchWorkspaceRequest>,
) -> Result<Json<SwitchWorkspaceResult>, AppError> {
    let domain_auth = crate::domains::auth::types::AuthContext::from(&auth);
    let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
    let result = sessions_context::switch_workspace(
        &state.db,
        &state.redis,
        &webauthn,
        state.config.auth_step_up_ttl_minutes,
        &domain_auth,
        workspace_id,
        SwitchWorkspaceInput {
            password: request.password,
            totp_code: request.totp_code,
            webauthn_response: request.webauthn_response,
            webauthn_challenge_id: request.webauthn_challenge_id,
            recovery_code: request.recovery_code,
        },
    )
    .await?;

    Ok(Json(result))
}
