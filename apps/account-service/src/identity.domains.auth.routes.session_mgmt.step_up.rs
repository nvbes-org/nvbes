use crate::domains::auth::types::{StepUpInput, StepUpResult};
use crate::domains::auth::verification;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use crate::{app::AppState, domains::auth::webauthn};
use axum::{
    Json,
    extract::{Extension, State},
    http::{HeaderMap, Uri, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub(crate) struct StepUpRequest {
    pow_nonce: String,
    pow_solution: String,
    password: Option<String>,
    totp_code: Option<String>,
    #[schema(value_type = Object)]
    webauthn_response: Option<webauthn_rs::prelude::PublicKeyCredential>,
    webauthn_challenge_id: Option<Uuid>,
    recovery_code: Option<String>,
}

#[utoipa::path(
    post,
    path = "/auth/step-up",
    tag = "auth",
    request_body = StepUpRequest,
    responses(
        (status = 200, description = "Step-up successful", body = StepUpResult),
        (status = 401, description = "Unauthorized or invalid credentials", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn step_up(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    uri: Uri,
    headers: HeaderMap,
    Json(request): Json<StepUpRequest>,
) -> Result<Response, AppError> {
    crate::domains::auth::challenge_proof::require_pow_solution(
        &state.db,
        Some(request.pow_nonce.as_str()),
        Some(request.pow_solution.as_str()),
    )
    .await?;

    let webauthn = webauthn::build_webauthn(&state.config)?;
    let browser_authenticated =
        crate::http::request::authorization_bearer_token(&headers)?.is_none();
    let result = verification::step_up(
        &state.db,
        &state.redis,
        state.config.auth_step_up_ttl_minutes,
        &webauthn,
        &auth,
        StepUpInput {
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
