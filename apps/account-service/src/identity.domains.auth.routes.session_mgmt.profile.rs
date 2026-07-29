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
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use std::time::Duration;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct AvatarUploadInput {
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AvatarUploadResult {
    pub upload_url: String,
    pub object_key: String,
}

#[utoipa::path(
    post,
    path = "/auth/me/avatar",
    tag = "auth",
    request_body = AvatarUploadInput,
    responses(
        (status = 200, description = "Profile avatar upload prepared", body = AvatarUploadResult),
        (status = 400, description = "Unsupported avatar type or size", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 500, description = "Storage error", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_avatar_upload(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<AvatarUploadInput>,
) -> Result<Json<AvatarUploadResult>, AppError> {
    if !["image/jpeg", "image/png", "image/webp"].contains(&input.content_type.as_str())
        || !(1..=5 * 1024 * 1024).contains(&input.size_bytes)
    {
        return Err(AppError::bad_request(
            "invalid_avatar",
            "Use a JPEG, PNG, or WebP image of 5 MB maximum.",
        ));
    }
    let object_key = format!(
        "account/profile-avatars/{}/{}",
        auth.user_id(),
        uuid::Uuid::new_v4()
    );
    let upload = state
        .storage
        .presign_upload(
            &object_key,
            Some(&input.content_type),
            input.size_bytes,
            Duration::from_secs(600),
        )
        .await
        .map_err(|_| {
            AppError::internal("storage_error", "Could not prepare profile photo upload.")
        })?;
    auth_db::save_profile_avatar(&state.db, auth.user_id(), &object_key).await?;
    Ok(Json(AvatarUploadResult {
        upload_url: upload.url,
        object_key,
    }))
}

#[utoipa::path(
    get,
    path = "/auth/me/avatar",
    tag = "auth",
    responses(
        (status = 307, description = "Temporary redirect to the profile avatar"),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Profile avatar not found"),
        (status = 500, description = "Storage error", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_avatar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Response, AppError> {
    let Some(object_key) = auth_db::fetch_profile_avatar(&state.db, auth.user_id()).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let download = state
        .storage
        .presign_download(&object_key, Duration::from_secs(300))
        .await
        .map_err(|_| AppError::internal("storage_error", "Could not prepare profile photo."))?;
    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(header::LOCATION, download.url)
        .body(axum::body::Body::empty())
        .map_err(|_| AppError::internal("response_error", "Failed to serve profile photo."))
}

#[utoipa::path(
    delete,
    path = "/auth/me/avatar",
    tag = "auth",
    responses(
        (status = 200, description = "Profile avatar deleted"),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 500, description = "Storage error", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_avatar_delete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(object_key) = auth_db::fetch_profile_avatar(&state.db, auth.user_id()).await? {
        state
            .storage
            .delete_objects(&[object_key])
            .await
            .map_err(|_| AppError::internal("storage_error", "Could not remove profile photo."))?;
    }
    auth_db::clear_profile_avatar(&state.db, auth.user_id()).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

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
