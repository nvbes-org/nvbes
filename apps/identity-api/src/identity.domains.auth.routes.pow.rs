use crate::app::AppState;
use crate::http::error::AppError;
use axum::{Json, Router, extract::State, routing::get};

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/pow", get(challenge_pow))
}

#[utoipa::path(
    get,
    path = "/auth/challenge/pow",
    tag = "auth",
    responses(
        (status = 200, description = "PoW challenge", body = crate::domains::auth::pow::PowChallenge),
    ),
)]
pub(crate) async fn challenge_pow(
    State(state): State<AppState>,
) -> Result<Json<crate::domains::auth::pow::PowChallenge>, AppError> {
    let challenge = crate::domains::auth::pow::issue_challenge(
        &state.db,
        state.config.auth_pow_difficulty,
        state.config.auth_pow_ttl_seconds,
    )
    .await?;
    Ok(Json(challenge))
}
