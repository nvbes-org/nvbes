use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, HeaderName},
};

use crate::{
    app::AppState,
    auth::AuthenticatedPrincipal,
    error::AppError,
    privacy_models::{GpcStatus, SuccessResponse},
};

const SEC_GPC: HeaderName = HeaderName::from_static("sec-gpc");

#[utoipa::path(
    get,
    path = "/api/v1/privacy/gpc",
    tag = "privacy",
    operation_id = "getAccountGpcStatus",
    security(("identityOAuth2" = ["account:legal:read"])),
    responses(
        (status = 200, body = GpcStatus),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_gpc_status(
    Extension(_auth): Extension<AuthenticatedPrincipal>,
    headers: HeaderMap,
) -> Json<GpcStatus> {
    let enabled = headers
        .get(SEC_GPC)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "1");
    Json(GpcStatus {
        gpc_enabled: enabled,
        gpc_opt_out_active: enabled,
    })
}

#[utoipa::path(
    post,
    path = "/api/v1/privacy/export",
    tag = "privacy",
    operation_id = "requestAccountPrivacyExport",
    security(("identityOAuth2" = ["account:export"])),
    responses(
        (status = 200, body = SuccessResponse),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn request_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<SuccessResponse>, AppError> {
    let document = crate::privacy_db::build_export(&state.db, auth.principal_id).await?;
    crate::privacy_db::store_export(&state.db, auth.principal_id, document).await?;
    Ok(Json(SuccessResponse::ok()))
}

#[utoipa::path(
    get,
    path = "/api/v1/privacy/export",
    tag = "privacy",
    operation_id = "downloadAccountPrivacyExport",
    security(("identityOAuth2" = ["account:export"])),
    responses(
        (status = 200, description = "Account-owned privacy export", content_type = "application/json"),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
        (status = 404, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn download_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<serde_json::Value>, AppError> {
    let document = crate::privacy_db::latest_export(&state.db, auth.principal_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "privacy_export_not_found",
                "No non-expired Account privacy export is available.",
            )
        })?;
    Ok(Json(document))
}
