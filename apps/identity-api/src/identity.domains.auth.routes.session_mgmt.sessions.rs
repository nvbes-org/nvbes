use crate::domains::auth::sessions;
use crate::domains::auth::sessions_mgmt;
use crate::domains::auth::types::LogoutResult;
use crate::domains::auth::types::StepUpSubject;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use crate::{app::AppState, domains::auth::verification};
use axum::{
    Json,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "Logout successful", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn logout(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    sessions::logout_browser_sessions(&state.db, &state.redis, &state.jwt, &auth, &headers).await?;
    let result = LogoutResult { success: true };

    let mut response = (StatusCode::OK, Json(result)).into_response();
    let secure_cookie = state.config.environment != "development";

    for base in ["session", "token", "refresh_token", "csrf_token"] {
        let name = crate::http::cookies::auth_cookie_name(base, secure_cookie);
        response.headers_mut().append(
            SET_COOKIE,
            crate::http::cookies::auth_cookie(&name, "", 0, secure_cookie)?,
        );
    }
    nvbes_core::security::insert_clear_site_data_header(response.headers_mut());

    Ok(response)
}

#[utoipa::path(
    get,
    path = "/auth/sessions",
    tag = "auth",
    responses(
        (status = 200, description = "List of active sessions", body = crate::domains::auth::types::SessionsResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn list_sessions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::SessionsResult>, AppError> {
    let result = sessions_mgmt::list(&state.redis, auth.user_id(), auth.session_id()).await?;
    Ok(Json(result))
}

#[utoipa::path(
    delete,
    path = "/auth/sessions/{sessionId}",
    tag = "auth",
    responses(
        (status = 200, description = "Session revoked", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Session not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_session(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(session_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<crate::domains::auth::types::LogoutResult>, AppError> {
    verification::require_recent_step_up(&state.redis, &auth, None).await?;
    sessions_mgmt::revoke(&state.db, &state.redis, auth.user_id(), session_id).await?;
    Ok(Json(LogoutResult { success: true }))
}

#[utoipa::path(
    post,
    path = "/auth/sessions/revoke-others",
    tag = "auth",
    responses(
        (status = 200, description = "Other sessions revoked", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_all_other_sessions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::LogoutResult>, AppError> {
    sessions_mgmt::revoke_all_others(&state.db, &state.redis, auth.user_id(), auth.session_id())
        .await?;
    Ok(Json(LogoutResult { success: true }))
}
