use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, StatusCode},
};
use uuid::Uuid;

use crate::{
    app::AppState, auth::AuthenticatedPrincipal, error::AppError, privacy_models::GpcStatus,
};

const SEC_GPC: HeaderName = HeaderName::from_static("sec-gpc");

#[utoipa::path(get, path = "/api/v1/privacy/gpc", tag = "privacy",
    security(("identityOAuth2" = ["account:legal:read"])),
    responses((status = 200, body = GpcStatus), (status = 401, body = crate::error::ErrorEnvelope),
      (status = 403, body = crate::error::ErrorEnvelope)))]
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

#[utoipa::path(post, path = "/api/v1/privacy/exports", tag = "privacy",
    operation_id = "requestAccountPrivacyExport",
    security(("identityOAuth2" = ["account:export"])),
    responses((status = 202, body = crate::privacy_db::ExportRequest),
      (status = 401, body = crate::error::ErrorEnvelope), (status = 403, body = crate::error::ErrorEnvelope)))]
pub async fn request_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<(StatusCode, Json<crate::privacy_db::ExportRequest>), AppError> {
    let export = crate::privacy_db::request(&state.db, auth.principal_id).await?;
    Ok((StatusCode::ACCEPTED, Json(export)))
}

#[utoipa::path(get, path = "/api/v1/privacy/exports/latest", tag = "privacy",
    operation_id = "getLatestAccountPrivacyExport",
    security(("identityOAuth2" = ["account:export"])),
    responses((status = 200, body = crate::privacy_db::ExportStatus),
      (status = 401, body = crate::error::ErrorEnvelope), (status = 403, body = crate::error::ErrorEnvelope),
      (status = 404, body = crate::error::ErrorEnvelope)))]
pub async fn get_latest_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<crate::privacy_db::ExportStatus>, AppError> {
    Ok(Json(
        crate::privacy_db::latest(&state.db, auth.principal_id).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/privacy/exports/{exportId}/document", tag = "privacy",
    operation_id = "downloadAccountPrivacyExport", params(("exportId" = Uuid, Path)),
    security(("identityOAuth2" = ["account:export"])),
    responses((status = 200, description = "Multi-product privacy export", content_type = "application/json"),
      (status = 401, body = crate::error::ErrorEnvelope), (status = 403, body = crate::error::ErrorEnvelope),
      (status = 404, body = crate::error::ErrorEnvelope)))]
pub async fn download_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    Path(export_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(
        crate::privacy_db::document(&state.db, auth.principal_id, export_id).await?,
    ))
}
