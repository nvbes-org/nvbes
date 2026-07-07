use crate::app::AppState;
use crate::http::error::AppError;
use axum::{Json, Router, extract::State, http::HeaderMap, response::Response, routing::post};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::{IdentifierRequest, IdentifierResult};

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/identifier", post(challenge_identifier))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/identifier",
    tag = "auth",
    request_body = IdentifierRequest,
    responses(
        (status = 200, description = "Identifier verified", body = IdentifierResult),
        (status = 403, description = "Risk policy blocked or bot guard failed", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_identifier(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<IdentifierRequest>,
) -> Result<Response, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_identifier",
        &meta.rate_limit_ip_key(),
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;

    super::identifier_guard::enforce_identifier_request_guards(
        &state.db,
        &state.config,
        &headers,
        &meta,
        &request,
    )
    .await?;

    let challenge =
        super::identifier_flow::resolve_uniform_identifier_challenge(&state.db, &request.email)
            .await?;

    super::challenge_response(
        &state.redis,
        axum::http::StatusCode::OK,
        challenge.principal_id,
        &request.email,
        &challenge.next_step,
        request.device_fingerprint,
        challenge.available_methods,
    )
    .await
}
