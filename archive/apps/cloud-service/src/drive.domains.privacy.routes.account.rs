use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
};
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{app::AppState, http::error::AppError};

use crate::domains::privacy::service::PrivacyRequestStatusResponse;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new().route("/privacy/requests/{requestId}", get(get_privacy_request))
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
