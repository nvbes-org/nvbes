use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
};

use crate::{
    app::AppState,
    auth::AuthenticatedPrincipal,
    error::AppError,
    profile_models::{AccountProfileEnvelope, UpdateProfileInput},
};

#[utoipa::path(
    get,
    path = "/api/v1/profile",
    tag = "profile",
    operation_id = "getAccountProfile",
    security(("identityOAuth2" = ["account:profile:read"])),
    responses(
        (status = 200, body = AccountProfileEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<AccountProfileEnvelope>, AppError> {
    let user = crate::profile_db::fetch_or_initialize(&state.db, auth.principal_id).await?;
    Ok(Json(AccountProfileEnvelope { user }))
}

#[utoipa::path(
    patch,
    path = "/api/v1/profile",
    tag = "profile",
    operation_id = "updateAccountProfile",
    security(("identityOAuth2" = ["account:profile:write"])),
    request_body = UpdateProfileInput,
    responses(
        (status = 200, body = AccountProfileEnvelope),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
        (status = 409, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn update_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<UpdateProfileInput>, JsonRejection>,
) -> Result<Json<AccountProfileEnvelope>, AppError> {
    let Json(input) = payload.map_err(AppError::invalid_json)?;
    let input = crate::profile_validation::validate(input)?;
    let user = crate::profile_db::update(&state.db, auth.principal_id, input).await?;
    Ok(Json(AccountProfileEnvelope { user }))
}
