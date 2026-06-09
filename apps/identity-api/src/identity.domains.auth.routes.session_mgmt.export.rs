use crate::app::AppState;
use crate::domains::auth::types::{DataExportResult, StepUpSubject};
use crate::domains::auth::verification;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use axum::{
    Json,
    body::Body,
    extract::{Extension, State},
    http::{HeaderValue, header},
    response::Response,
};

#[utoipa::path(
    post,
    path = "/auth/me/export",
    tag = "auth",
    responses(
        (status = 200, description = "Data export request recorded", body = DataExportResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DataExportResult>, AppError> {
    let domain_auth = crate::domains::auth::types::AuthContext::from(&auth);
    crate::domains::auth::data_export::request_account_export(
        &state.db,
        &state.redis,
        &domain_auth,
    )
    .await?;
    Ok(Json(DataExportResult { success: true }))
}

#[utoipa::path(
    get,
    path = "/auth/me/export",
    tag = "auth",
    responses(
        (status = 200, description = "Prepared personal data export", content_type = "application/json"),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "No prepared export available", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_export_download(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Response, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_me_export_download",
        &format!("user:{}", auth.user_id()),
        12,
        std::time::Duration::from_secs(3600),
    )
    .await?;

    verification::require_recent_step_up(&state.redis, &auth, None).await?;

    let export =
        crate::domains::auth::data_export::load_account_export(&state.redis, auth.user_id())
            .await?
            .ok_or_else(|| {
                AppError::not_found(
                    "data_export_not_ready",
                    "No prepared data export is available. Request a new export first.",
                )
            })?;

    let body = serde_json::to_vec_pretty(&export)?;
    let filename = format!("nvbes-identity-export-{}.json", auth.user_id());
    let content_disposition =
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|error| AppError::internal("invalid_export_filename", error.to_string()))?;

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .body(Body::from(body))
        .map_err(|error| AppError::internal("data_export_response_failed", error.to_string()))
}
