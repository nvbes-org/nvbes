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
        crate::domains::auth::bot_response::record_decision("block", "decoy_link_clicked");
        crate::domains::auth::bot_response::apply_tarpit(1.0).await;
        return Err(AppError::forbidden(
            "decoy_link_clicked",
            "Bot detected: decoy link was clicked.",
        ));
    }

    if let Some(proof) = &request.bot_guard {
        if let Err(error) =
            crate::domains::auth::bot_guard::verify(proof, "login_identifier", &config.jwt_secret)
        {
            crate::domains::auth::bot_response::record_decision("block", "bot_guard_proof");
            crate::domains::auth::bot_response::apply_tarpit(1.0).await;
            return Err(error);
        }
    }

    enforce_bot_score(headers, meta, request).await?;
    Ok(())
}

async fn enforce_bot_score(
    headers: &HeaderMap,
    meta: &LoginRequestMeta,
    request: &IdentifierRequest,
) -> Result<(), AppError> {
    let ip = meta.ip().unwrap_or_else(|| "unknown".to_string());
    let http_scores = crate::domains::auth::http_signals::extract_http_signals(headers, &ip)?;
    let ua_client_hints =
        crate::domains::auth::ua_client_hints::UserAgentClientHints::from_headers(headers);
    let ua_ch_assessment = ua_client_hints.assess_consistency(
        meta.user_agent().as_deref(),
        request.bot_signals.as_ref(),
        request.device_fingerprint.as_ref(),
    );
    let ua_assessment = crate::domains::auth::user_agent::risk::assess(
        meta.user_agent().as_deref(),
        ua_client_hints.brands.as_deref(),
        request.bot_signals.as_ref(),
    );
    let mut bot_score =
        (http_scores.total() + ua_ch_assessment.score + ua_assessment.score).min(1.0);
    let mut bot_factors = json!({
        "interval_score": http_scores.interval_score,
        "header_order_score": http_scores.header_order_score,
        "ua_language_score": http_scores.ua_language_score,
        "ua_ch_score": ua_ch_assessment.score,
        "ua_ch_factors": ua_ch_assessment.factors,
        "ua_score": ua_assessment.score,
        "ua_factors": ua_assessment.factors,
        "client": ua_assessment.client,
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
        crate::domains::auth::bot_response::record_decision("block", "bot_score_high");
        crate::domains::auth::bot_response::apply_tarpit(bot_score).await;
        tracing::warn!(bot_score, ip = %ip, "Bot guard: request blocked");
        return Err(AppError::forbidden(
            "bot_detected",
            "Automated request detected.",
        ));
    }
    if bot_score >= 0.60 {
        crate::domains::auth::bot_response::record_decision("monitor", "bot_score_elevated");
        tracing::info!(bot_score, ip = %ip, "Bot guard: elevated risk, monitoring");
    } else {
        crate::domains::auth::bot_response::record_decision("allow", "bot_score_low");
    }

    tracing::debug!(bot_score, factors = %bot_factors, "Bot guard score computed");
    Ok(())
}
