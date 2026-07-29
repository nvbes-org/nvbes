use crate::app::AppState;
use crate::domains::auth::sessions_context;
use crate::domains::auth::types::{SwitchWorkspaceInput, SwitchWorkspaceResult};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, Uri, header::SET_COOKIE},
    response::{IntoResponse, Response},
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
    uri: Uri,
    headers: HeaderMap,
    Json(request): Json<SwitchWorkspaceRequest>,
) -> Result<Response, AppError> {
    let domain_auth = crate::domains::auth::types::AuthContext::from(&auth);
    let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
    let browser_authenticated =
        crate::http::request::authorization_bearer_token(&headers)?.is_none();
    let result = sessions_context::switch_workspace(
        &state.db,
        &state.redis,
        &webauthn,
        &state.config,
        &domain_auth,
        workspace_id,
        SwitchWorkspaceInput {
            password: request.password,
            totp_code: request.totp_code,
            webauthn_response: request.webauthn_response,
            webauthn_challenge_id: request.webauthn_challenge_id,
            recovery_code: request.recovery_code,
        },
        browser_authenticated,
    )
    .await?;

    let browser_session_token = result.browser_session_token.clone();
    let mut response = Json(result).into_response();
    if let Some(browser_session_token) = browser_session_token {
        let authuser = crate::http::authuser::from_uri_and_headers(&uri, &headers)?;
        let secure_cookie = state.config.environment != "development";
        let max_age = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
        let session_cookie_name =
            crate::http::cookies::auth_cookie_name_with_user("session", &authuser, secure_cookie);
        let csrf_cookie_name = crate::http::cookies::auth_cookie_name_with_user(
            "csrf_token",
            &authuser,
            secure_cookie,
        );
        let csrf_token = crate::http::cookies::generate_csrf_token(
            &browser_session_token,
            &state.config.jwt_secret,
        );
        response.headers_mut().append(
            SET_COOKIE,
            crate::http::cookies::auth_cookie(
                &session_cookie_name,
                &browser_session_token,
                max_age,
                secure_cookie,
            )?,
        );
        response.headers_mut().append(
            SET_COOKIE,
            crate::http::cookies::csrf_cookie(
                &csrf_cookie_name,
                &csrf_token,
                max_age,
                secure_cookie,
            )?,
        );
    }

    Ok(response)
}
