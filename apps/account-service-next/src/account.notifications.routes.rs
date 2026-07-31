use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
};

use crate::{
    app::AppState, auth::AuthenticatedPrincipal, error::AppError,
    settings_models::AccountNotifications,
};

#[utoipa::path(
    get,
    path = "/api/v1/notifications",
    tag = "settings",
    operation_id = "getAccountNotifications",
    security(("identityOAuth2" = ["account:preferences:read"])),
    responses(
        (status = 200, body = AccountNotifications),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_notifications(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<AccountNotifications>, AppError> {
    let notifications =
        crate::notifications_db::fetch_or_initialize(&state.db, auth.principal_id).await?;
    Ok(Json(notifications))
}

#[utoipa::path(
    put,
    path = "/api/v1/notifications",
    tag = "settings",
    operation_id = "updateAccountNotifications",
    security(("identityOAuth2" = ["account:preferences:write"])),
    request_body = AccountNotifications,
    responses(
        (status = 200, body = AccountNotifications),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn update_notifications(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<AccountNotifications>, JsonRejection>,
) -> Result<Json<AccountNotifications>, AppError> {
    let Json(input) = payload.map_err(AppError::invalid_json)?;
    let notifications =
        crate::notifications_db::update(&state.db, auth.principal_id, input).await?;
    Ok(Json(notifications))
}
