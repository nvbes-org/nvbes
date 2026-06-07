use crate::app::AppState;
use crate::http::middleware::jwt::AuthContext;
use axum::{
    Json,
    extract::{Extension, State},
};

use super::profile;

pub(crate) async fn me_update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::UpdateProfileInput>,
) -> Result<Json<crate::domains::auth::types::UpdateProfileResult>, crate::http::error::AppError> {
    profile::me_update(State(state), Extension(auth), Json(request)).await
}

pub(crate) async fn me_preferences_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::UserPreferences>, crate::http::error::AppError> {
    profile::me_preferences_get(State(state), Extension(auth)).await
}

pub(crate) async fn me_preferences_put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::UserPreferences>,
) -> Result<Json<crate::domains::auth::types::UserPreferences>, crate::http::error::AppError> {
    profile::me_preferences_put(State(state), Extension(auth), Json(request)).await
}

pub(crate) async fn me_notifications_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::UserNotifications>, crate::http::error::AppError> {
    profile::me_notifications_get(State(state), Extension(auth)).await
}

pub(crate) async fn me_notifications_put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::UserNotifications>,
) -> Result<Json<crate::domains::auth::types::UserNotifications>, crate::http::error::AppError> {
    profile::me_notifications_put(State(state), Extension(auth), Json(request)).await
}
