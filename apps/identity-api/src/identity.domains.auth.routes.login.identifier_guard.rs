use axum::http::HeaderMap;
use serde_json::json;

use crate::app::AppConfig;
use crate::database::Database;
use crate::http::error::AppError;

use super::{LoginRequestMeta, types::IdentifierRequest};

pub(crate) async fn enforce_identifier_request_guards(
    db: &Database,
    config: &AppConfig,
    headers: &HeaderMap,
    meta: &LoginRequestMeta,
    request: &IdentifierRequest,
) -> Result<(), AppError> {
    crate::domains::auth::challenge_proof::require_pow_solution(
        db,
        Some(request.pow_nonce.as_str()),
        Some(request.pow_solution.as_str()),
    )
    .await?;

    if request.decoy_link_clicked == Some(true) {
        return Err(AppError::forbidden(
            "decoy_link_clicked",
            "Bot detected: decoy link was clicked.",
        ));
    }

    if let Some(proof) = &request.bot_guard {
        crate::domains::auth::bot_guard::verify(proof, "login_identifier", &config.jwt_secret)?;
    }

    enforce_bot_score(headers, meta, request)?;
    Ok(())
}

fn enforce_bot_score(
    headers: &HeaderMap,
    meta: &LoginRequestMeta,
    request: &IdentifierRequest,
) -> Result<(), AppError> {
    let ip = meta.ip().unwrap_or_else(|| "unknown".to_string());
    let http_scores = crate::domains::auth::http_signals::extract_http_signals(headers, &ip)?;
    let mut bot_score = http_scores.total();
    let mut bot_factors = json!({
        "interval_score": http_scores.interval_score,
        "header_order_score": http_scores.header_order_score,
        "ua_language_score": http_scores.ua_language_score,
    });

    if let Some(signals) = &request.bot_signals {
        let client_score = crate::domains::auth::bot_scorer::score_client_signals(signals);
        bot_score = (bot_score + client_score.score).min(1.0);
        if let serde_json::Value::Object(ref mut map) = bot_factors {
            map.insert("client_factors".to_string(), client_score.factors);
            map.insert("client_score".to_string(), json!(client_score.score));
        }
    }

    if bot_score >= 0.80 {
        tracing::warn!(bot_score, ip = %ip, "Bot guard: request blocked");
        return Err(AppError::forbidden(
            "bot_detected",
            "Automated request detected.",
        ));
    }
    if bot_score >= 0.60 {
        tracing::info!(bot_score, ip = %ip, "Bot guard: elevated risk, monitoring");
    }

    tracing::debug!(bot_score, factors = %bot_factors, "Bot guard score computed");
    Ok(())
}
