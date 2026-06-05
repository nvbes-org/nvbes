use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{app::AppState, http::error::AppError};

use crate::domains::privacy::service::{PrivacyRequestResponse, PrivacyRequestStatusResponse};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/privacy/export", post(export_account_data))
        .route("/privacy/delete-account", post(delete_account))
        .route("/privacy/requests/{requestId}", get(get_privacy_request))
}

#[derive(Deserialize, ToSchema)]
struct DeleteAccountRequest {
    confirmation: String,
}

#[utoipa::path(
    get,
    path = "/privacy/requests/{requestId}",
    tag = "privacy",
    params(
        ("requestId" = Uuid, Path, description = "Privacy request ID"),
    ),
    responses(
        (status = 200, description = "Privacy request status", body = PrivacyRequestStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn get_privacy_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
) -> Result<Json<PrivacyRequestStatusResponse>, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;
    let result =
        crate::domains::privacy::get_request(&state.db, state.storage.as_ref(), &auth, request_id)
            .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/privacy/export",
    tag = "privacy",
    responses(
        (status = 200, description = "Account export requested", body = PrivacyRequestResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn export_account_data(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PrivacyRequestResponse>, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;
    crate::domains::auth::require_recent_step_up(&state.db, &auth).await?;
    let result =
        crate::domains::privacy::request_account_export(&state.db, &state.redis, &auth).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/privacy/delete-account",
    tag = "privacy",
    request_body = DeleteAccountRequest,
    responses(
        (status = 200, description = "Account deletion requested", body = PrivacyRequestResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeleteAccountRequest>,
) -> Result<Json<PrivacyRequestResponse>, AppError> {
    if request.confirmation != "DELETE_MY_ACCOUNT" {
        return Err(AppError::bad_request(
            "confirmation_required",
            "Account deletion requires confirmation value DELETE_MY_ACCOUNT.",
        ));
    }

    let token = crate::http::request::bearer_token(&headers)?;
    let auth = crate::domains::auth::authenticate(&state.db, &token, Some(&headers)).await?;
    crate::domains::auth::require_recent_step_up(&state.db, &auth).await?;
    let result =
        crate::domains::privacy::request_account_delete(&state.db, &state.redis, &auth).await?;
    Ok(Json(result))
}
