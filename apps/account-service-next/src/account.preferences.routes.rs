use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
};

use crate::{
    app::AppState, auth::AuthenticatedPrincipal, error::AppError,
    settings_models::AccountPreferences,
};

#[utoipa::path(
    get,
    path = "/api/v1/preferences",
    tag = "settings",
    operation_id = "getAccountPreferences",
    security(("identityOAuth2" = ["account:preferences:read"])),
    responses(
        (status = 200, body = AccountPreferences),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_preferences(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<AccountPreferences>, AppError> {
    let preferences =
        crate::preferences_db::fetch_or_initialize(&state.db, auth.principal_id).await?;
    Ok(Json(preferences))
}

#[utoipa::path(
    put,
    path = "/api/v1/preferences",
    tag = "settings",
    operation_id = "updateAccountPreferences",
    security(("identityOAuth2" = ["account:preferences:write"])),
    request_body = AccountPreferences,
    responses(
        (status = 200, body = AccountPreferences),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn update_preferences(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<AccountPreferences>, JsonRejection>,
) -> Result<Json<AccountPreferences>, AppError> {
    let Json(input) = payload.map_err(AppError::invalid_json)?;
    let preferences = crate::preferences_db::update(&state.db, auth.principal_id, input).await?;
    Ok(Json(preferences))
}
