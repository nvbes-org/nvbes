use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};

use super::login::LoginRequestMeta;
use crate::app::AppState;
use crate::http::error::AppError;

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
    headers: HeaderMap,
) -> Result<Json<crate::domains::auth::pow::PowChallenge>, AppError> {
    let meta = LoginRequestMeta::from_headers(&headers);
    let ip = meta.ip().unwrap_or_else(|| "unknown".to_string());

    let http_scores = crate::domains::auth::http_signals::extract_http_signals(&headers, &ip).ok();
    let http_total = http_scores.as_ref().map(|s| s.total()).unwrap_or(0.0);

    let ua_client_hints =
        crate::domains::auth::ua_client_hints::UserAgentClientHints::from_headers(&headers);
    let ua_ch_assessment =
        ua_client_hints.assess_consistency(meta.user_agent().as_deref(), None, None);
    let ua_assessment = crate::domains::auth::user_agent::risk::assess(
        meta.user_agent().as_deref(),
        ua_client_hints.brands.as_deref(),
        None,
    );

    let bot_score = (http_total + ua_ch_assessment.score + ua_assessment.score).min(1.0);
    let risk_score = bot_score * 100.0;

    let challenge = crate::domains::auth::pow::issue_progressive_challenge(
        &state.db,
        state.config.auth_pow_difficulty,
        0,
        risk_score,
        state.config.auth_pow_ttl_seconds,
    )
    .await?;
    Ok(Json(challenge))
}
