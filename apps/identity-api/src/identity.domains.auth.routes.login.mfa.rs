use crate::app::AppState;
use crate::domains::auth::state::{delete_state, fetch_state};
use crate::domains::auth::{login_challenges, sessions};
use crate::http::cookies::{
    auth_cookie, auth_cookie_name_with_user, csrf_cookie, generate_csrf_token,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::MfaRequest;

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/mfa", post(challenge_mfa))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/mfa",
    tag = "auth",
    request_body = MfaRequest,
    responses(
        (status = 200, description = "Login successful after MFA", body = crate::domains::auth::types::LoginResult),
        (status = 401, description = "Invalid credentials or state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_mfa(
    State(state): State<AppState>,
    Query(query): Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<MfaRequest>,
) -> Result<Response, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login_mfa",
        &format!(
            "ip:{}",
            crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
        ),
        20,
        std::time::Duration::from_secs(60),
    )
    .await?;
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login_mfa",
        &format!("state:{}", request.state_token),
        8,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let auth_state = fetch_state(&state.redis, request.state_token, "mfa").await?;
    let principal_id = auth_state.principal_id.ok_or_else(|| {
        AppError::unauthorized(
            "invalid_auth_state",
            "The authentication session is invalid or has expired.",
        )
    })?;

    let selected_factor_count = usize::from(request.totp_code.is_some())
        + usize::from(request.recovery_code.is_some())
        + usize::from(request.webauthn_response.is_some());
    if selected_factor_count != 1 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Exactly one MFA method must be provided.",
        ));
    }

    let authenticated_method = if let Some(code) = request.totp_code {
        crate::domains::auth::mfa::verify_totp(&state.db, principal_id, &code).await?;
        "otp".to_string()
    } else if let Some(code) = request.recovery_code {
        crate::domains::auth::mfa::verify_recovery(&state.db, principal_id, &code).await?;
        "recovery".to_string()
    } else if let Some(credential) = request.webauthn_response.as_ref() {
        let challenge_id = request.webauthn_challenge_id.ok_or_else(|| {
            AppError::bad_request("validation_failed", "A WebAuthn challenge id is required.")
        })?;
        let webauthn = crate::domains::auth::webauthn::build_webauthn(&state.config)?;
        match crate::domains::auth::webauthn::finish_login_authentication(
            &state.db,
            &state.redis,
            &webauthn,
            auth_state.id,
            principal_id,
            challenge_id,
            credential,
        )
        .await
        {
            Ok(method) => method,
            Err(err) => {
                if let Ok(failed_attempts) = login_challenges::record_failed_attempt(
                    &state.redis,
                    challenge_id,
                    auth_state.id,
                    principal_id,
                    "webauthn_login",
                )
                .await
                {
                    if failed_attempts >= 5 {
                        return Err(AppError::forbidden(
                            "challenge_locked",
                            "The WebAuthn challenge has been locked after repeated failures.",
                        ));
                    }
                }
                return Err(err);
            }
        }
    } else {
        return Err(AppError::bad_request(
            "validation_failed",
            "A TOTP code, recovery code, or WebAuthn assertion is required.",
        ));
    };

    let result = sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.jwt,
        &state.config,
        principal_id,
        sessions::LoginSessionContext {
            email: auth_state.email,
            ip: crate::http::request::client_ip(&headers),
            user_agent: crate::http::request::user_agent(&headers),
            device_fingerprint: auth_state.device_fingerprint,
            amr: vec!["pwd".to_string(), authenticated_method],
            acr: "aal2",
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
