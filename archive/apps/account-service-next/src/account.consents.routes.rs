use std::net::SocketAddr;

use axum::{
    Extension, Json,
    extract::{
        ConnectInfo, Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::StatusCode,
};

use crate::{
    app::AppState,
    auth::AuthenticatedPrincipal,
    consents_models::{AccountConsent, ConsentHistory, ConsentInput, ConsentPage},
    error::AppError,
};

#[utoipa::path(
    get,
    path = "/api/v1/consents",
    tag = "privacy",
    operation_id = "listAccountConsents",
    security(("identityOAuth2" = ["account:legal:read"])),
    params(ConsentPage),
    responses(
        (status = 200, body = ConsentHistory),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn list_consents(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    query: Result<Query<ConsentPage>, QueryRejection>,
) -> Result<Json<ConsentHistory>, AppError> {
    let Query(page) = query.map_err(AppError::invalid_query)?;
    let (cursor, limit) = crate::consents_models::pagination(page)?;
    let history = crate::consents_db::list(&state.db, auth.principal_id, cursor, limit).await?;
    Ok(Json(history))
}

#[utoipa::path(
    post,
    path = "/api/v1/consents",
    tag = "privacy",
    operation_id = "grantAccountConsent",
    security(("identityOAuth2" = ["account:legal:write"])),
    request_body = ConsentInput,
    responses(
        (status = 200, body = AccountConsent),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn grant_consent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    payload: Result<Json<ConsentInput>, JsonRejection>,
) -> Result<Json<AccountConsent>, AppError> {
    let Json(input) = payload.map_err(AppError::invalid_json)?;
    let input = crate::consents_models::validate_input(input)?;
    let consent = crate::consents_db::grant(
        &state.db,
        auth.principal_id,
        input,
        Some(peer.ip().to_string()),
    )
    .await?;
    Ok(Json(consent))
}

#[utoipa::path(
    delete,
    path = "/api/v1/consents",
    tag = "privacy",
    operation_id = "revokeAccountConsent",
    security(("identityOAuth2" = ["account:legal:write"])),
    request_body = ConsentInput,
    responses(
        (status = 204, description = "Consent revoked"),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn revoke_consent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<ConsentInput>, JsonRejection>,
) -> Result<StatusCode, AppError> {
    let Json(input) = payload.map_err(AppError::invalid_json)?;
    let input = crate::consents_models::validate_input(input)?;
    crate::consents_db::revoke(&state.db, auth.principal_id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}
