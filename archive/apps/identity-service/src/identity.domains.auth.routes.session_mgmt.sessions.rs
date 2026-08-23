use crate::domains::auth::sessions;
use crate::domains::auth::sessions_mgmt;
use crate::domains::auth::types::LogoutResult;
use crate::domains::auth::types::StepUpSubject;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use crate::{app::AppState, domains::auth::verification};
use axum::{
    Json,
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

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
    sessions::logout_browser_sessions(&state.db, &state.redis, &auth, &headers).await?;
    let (distinct_id, session_id) = crate::http::request::product_analytics_correlation(&headers);
    state.product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::user(
            "auth.logout_completed",
            auth.user_id(),
        )
        .correlation(distinct_id, session_id),
    );
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
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<crate::domains::auth::types::SessionsResult>, AppError> {
    let result = sessions_mgmt::list(
        &state.redis,
        auth.user_id(),
        auth.session_id(),
        query.limit,
        query.cursor,
    )
    .await?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub(crate) struct ListSessionsQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
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
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<crate::domains::auth::types::LogoutResult>, AppError> {
    if auth.session_id() != session_id {
        verification::require_recent_phishing_resistant_step_up(&state.redis, &auth).await?;
    }
    sessions_mgmt::revoke(&state.db, &state.redis, auth.user_id(), session_id).await?;
    let (distinct_id, analytics_session_id) =
        crate::http::request::product_analytics_correlation(&headers);
    state.product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::user(
            "auth.session_revoked",
            auth.user_id(),
        )
        .correlation(distinct_id, analytics_session_id)
        .property("status", "single"),
    );
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
    headers: HeaderMap,
) -> Result<Json<crate::domains::auth::types::LogoutResult>, AppError> {
    verification::require_recent_account_step_up(&state.db, &state.redis, &auth).await?;
    sessions_mgmt::revoke_all_others(&state.db, &state.redis, auth.user_id(), auth.session_id())
        .await?;
    let (distinct_id, session_id) = crate::http::request::product_analytics_correlation(&headers);
    state.product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::user(
            "auth.session_revoked",
            auth.user_id(),
        )
        .correlation(distinct_id, session_id)
        .property("status", "others"),
    );
    Ok(Json(LogoutResult { success: true }))
}

#[utoipa::path(
    post,
    path = "/auth/sessions/revoke-all",
    tag = "auth",
    responses(
        (status = 200, description = "All sessions revoked", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Recent step-up required", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_all_sessions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Response, AppError> {
    verification::require_recent_step_up(&state.redis, &auth, None).await?;
    sessions_mgmt::revoke_all_user_sessions(&state.db, &state.redis, auth.user_id()).await?;

    let mut response = (StatusCode::OK, Json(LogoutResult { success: true })).into_response();
    nvbes_core::security::insert_clear_site_data_header(response.headers_mut());
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/auth/sessions/{sessionId}/confirm",
    tag = "auth",
    responses(
        (status = 200, description = "High-risk session explicitly confirmed", body = crate::domains::auth::types::SessionView),
        (status = 401, description = "Recent step-up required", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Session not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn confirm_high_risk_session(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(session_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<crate::domains::auth::types::SessionView>, AppError> {
    if auth.session_id() != session_id {
        return Err(AppError::forbidden(
            "session_confirmation_forbidden",
            "Only the current session can be confirmed.",
        ));
    }
    verification::require_recent_phishing_resistant_step_up(&state.redis, &auth).await?;
    let session = sessions_mgmt::confirm_high_risk_session(
        &state.db,
        &state.redis,
        auth.user_id(),
        session_id,
    )
    .await?;
    Ok(Json(session))
}
