use crate::domains::auth::sessions_mgmt;
use crate::domains::auth::types::StepUpSubject;
use crate::domains::auth::types::{
    DeleteAccountResult, MeResult, UpdateProfileInput, UpdateProfileResult, UserNotifications,
    UserPreferences,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use crate::{app::AppState, domains::auth::db as auth_db};
use axum::{
    Json,
    extract::{Extension, State},
};

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user info", body = crate::domains::auth::types::MeResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<MeResult>, AppError> {
    let user = auth_db::fetch_user_view(&state.db, auth.user_id()).await?;
    let session = sessions_mgmt::fetch_view(&state.redis, auth.session_id(), true).await?;
    Ok(Json(MeResult {
        user,
        current_tenant_id: session.tenant_id,
        current_organization_id: session.organization_id,
        current_workspace_id: session.workspace_id,
        current_workspace_region: session.workspace_region,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/me/delete",
    tag = "auth",
    responses(
        (status = 200, description = "Account deleted successfully", body = DeleteAccountResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_delete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeleteAccountResult>, AppError> {
    let domain_auth = crate::domains::auth::types::AuthContext::from(&auth);
    let result = crate::domains::auth::account_deletion::delete_account(
        &state.db,
        &state.redis,
        &domain_auth,
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    patch,
    path = "/auth/me",
    tag = "auth",
    request_body = UpdateProfileInput,
    responses(
        (status = 200, description = "Profile updated", body = UpdateProfileResult),
        (status = 400, description = "Validation error", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 409, description = "Username already taken", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<UpdateProfileInput>,
) -> Result<Json<UpdateProfileResult>, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_me_update",
        &format!("user:{}", auth.user_id()),
        10,
        std::time::Duration::from_secs(300),
    )
    .await?;

    let user = auth_db::update_user_profile(&state.db, auth.user_id(), &request).await?;
    Ok(Json(UpdateProfileResult { user }))
}

#[utoipa::path(
    get,
    path = "/auth/me/preferences",
    tag = "auth",
    responses(
        (status = 200, description = "User preferences", body = UserPreferences),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_preferences_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<UserPreferences>, AppError> {
    let prefs = auth_db::fetch_user_preferences(&state.db, auth.user_id()).await?;
    Ok(Json(prefs))
}

#[utoipa::path(
    put,
    path = "/auth/me/preferences",
    tag = "auth",
    request_body = UserPreferences,
    responses(
        (status = 200, description = "Preferences updated", body = UserPreferences),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_preferences_put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<UserPreferences>,
) -> Result<Json<UserPreferences>, AppError> {
    let prefs = auth_db::update_user_preferences(&state.db, auth.user_id(), &request).await?;
    Ok(Json(prefs))
}

#[utoipa::path(
    get,
    path = "/auth/me/notifications",
    tag = "auth",
    responses(
        (status = 200, description = "Notification preferences", body = UserNotifications),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_notifications_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<UserNotifications>, AppError> {
    let notifs = auth_db::fetch_user_notifications(&state.db, auth.user_id()).await?;
    Ok(Json(notifs))
}

#[utoipa::path(
    put,
    path = "/auth/me/notifications",
    tag = "auth",
    request_body = UserNotifications,
    responses(
        (status = 200, description = "Notifications updated", body = UserNotifications),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_notifications_put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<UserNotifications>,
) -> Result<Json<UserNotifications>, AppError> {
    let notifs = auth_db::update_user_notifications(&state.db, auth.user_id(), &request).await?;
    Ok(Json(notifs))
}
