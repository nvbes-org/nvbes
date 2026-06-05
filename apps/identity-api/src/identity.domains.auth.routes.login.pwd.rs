use crate::app::AppState;
use crate::domains::auth::{
    sessions,
    state::{delete_state, fetch_state},
};
use crate::http::cookies::{
    auth_cookie, auth_cookie_name_with_user, csrf_cookie, generate_csrf_token,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::{IdentifierResult, PwdRequest};

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/pwd", post(challenge_pwd))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/pwd",
    tag = "auth",
    request_body = PwdRequest,
    responses(
        (status = 200, description = "Login successful", body = crate::domains::auth::types::LoginResult),
        (status = 202, description = "Password accepted, MFA challenge required", body = IdentifierResult),
        (status = 401, description = "Invalid credentials or state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_pwd(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<PwdRequest>,
) -> Result<Response, AppError> {
    let auth_state = fetch_state(&state.redis, request.state_token, "pwd").await?;
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login",
        &format!("key:{}", auth_state.email),
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let login_input = crate::domains::auth::types::LoginInput {
        email: auth_state.email.clone(),
        password: request.password,
        ip: crate::http::request::client_ip(&headers),
        user_agent: crate::http::request::user_agent(&headers),
        device_fingerprint: auth_state.device_fingerprint.clone(),
    };
    let verified =
        sessions::verify_primary_credentials(&state.db, &state.redis, &state.config, &login_input)
            .await?;
    let requires_mfa =
        crate::domains::auth::mfa::has_active_factor(&state.db, verified.principal_id).await?;

    if requires_mfa {
        let available_methods =
            crate::domains::auth::mfa::list_login_methods(&state.db, verified.principal_id).await?;
        let state_id = crate::domains::auth::state::create_state(
            &state.redis,
            Some(verified.principal_id),
            &auth_state.email,
            "mfa",
            auth_state.device_fingerprint,
            None,
        )
        .await?;
        delete_state(&state.redis, request.state_token).await?;
        return Ok((
            StatusCode::ACCEPTED,
            Json(IdentifierResult {
                next_step: "mfa".to_string(),
                state_token: state_id,
                available_methods: Some(available_methods),
            }),
        )
            .into_response());
    }

    let result = sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.jwt,
        &state.config,
        verified.principal_id,
        sessions::LoginSessionContext {
            email: verified.email,
            ip: login_input.ip,
            user_agent: login_input.user_agent,
            device_fingerprint: auth_state.device_fingerprint,
            amr: vec!["pwd".to_string()],
            acr: "aal1",
        },
    )
    .await?;

    let secure_cookie = state.config.environment != "development";
    let authuser = query.authuser.as_deref().unwrap_or("0");
    let session_cookie_name = auth_cookie_name_with_user("session", authuser, secure_cookie);
    let session_cookie_value = result.session_token.clone();
    let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
    let csrf_token = generate_csrf_token();
    let csrf_cookie_name = auth_cookie_name_with_user("csrf_token", authuser, secure_cookie);
    let mut response = (StatusCode::OK, Json(result)).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        auth_cookie(
            &session_cookie_name,
            &session_cookie_value,
            session_expires_in,
            secure_cookie,
        )?,
    );
    response.headers_mut().append(
        SET_COOKIE,
        csrf_cookie(
            &csrf_cookie_name,
            &csrf_token,
            session_expires_in,
            secure_cookie,
        )?,
    );
    delete_state(&state.redis, request.state_token).await?;
    Ok(response)
}
