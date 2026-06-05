use crate::app::AppState;
use crate::domains::auth::state::create_state;
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;
use serde_json::json;

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
        (status = 403, description = "Risk policy blocked or Turnstile failed", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_identifier(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<IdentifierRequest>,
) -> Result<Response, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_identifier",
        &format!(
            "ip:{}",
            crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string())
        ),
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;

    if state.config.auth_pow_enabled {
        let nonce = request
            .pow_nonce
            .as_deref()
            .ok_or_else(|| AppError::bad_request("pow_missing", "PoW challenge required."))?;
        let solution = request
            .pow_solution
            .as_deref()
            .ok_or_else(|| AppError::bad_request("pow_missing", "PoW solution required."))?;
        crate::domains::auth::pow::verify_solution(&state.db, nonce, solution).await?;
    }

    if request.decoy_link_clicked == Some(true) {
        return Err(AppError::forbidden(
            "decoy_link_clicked",
            "Bot detected: decoy link was clicked.",
        ));
    }

    if let Some(secret) = &state.config.turnstile_secret_key {
        if let Some(token) = request.turnstile_token {
            let ip = crate::http::request::client_ip(&headers);
            let domain = state.config.webauthn_rp_id.clone();
            crate::domains::auth::turnstile::verify_token(
                secret,
                &token,
                crate::domains::auth::turnstile::VerifyOptions {
                    ip: ip.as_deref(),
                    expected_action: Some("login_identifier"),
                    expected_hostname: Some(&domain),
                    idempotency_key: None,
                },
            )
            .await?;
        } else {
            return Err(AppError::forbidden(
                "missing_turnstile",
                "Turnstile token required",
            ));
        }
    }

    if let Some(proof) = &request.bot_guard {
        crate::domains::auth::bot_guard::verify(
            proof,
            "login_identifier",
            &state.config.jwt_secret,
        )?;
    }

    let ip_str = crate::http::request::client_ip(&headers).unwrap_or_else(|| "unknown".to_string());
    let http_scores = crate::domains::auth::http_signals::extract_http_signals(&headers, &ip_str)?;
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
        tracing::warn!(bot_score, ip = %ip_str, "Bot guard: request blocked");
        return Err(AppError::forbidden(
            "bot_detected",
            "Automated request detected.",
        ));
    } else if bot_score >= 0.60 {
        tracing::info!(bot_score, ip = %ip_str, "Bot guard: elevated risk, monitoring");
    }

    tracing::debug!(bot_score, factors = %bot_factors, "Bot guard score computed");

    let principal_id =
        crate::domains::auth::password::db::find_principal_and_display_name_by_email(
            &state.db,
            &request.email,
        )
        .await?
        .map(|(principal_id, _display_name)| principal_id);

    let mut next_step = "pwd".to_string();
    let mut available_methods = None;

    if let Some(pid) = principal_id {
        if let Ok(prefs) = crate::domains::auth::db::fetch_user_preferences(&state.db, pid).await {
            if prefs.skip_password {
                if let Ok(methods) =
                    crate::domains::auth::mfa::list_login_methods(&state.db, pid).await
                {
                    if methods.contains(&"webauthn".to_string()) {
                        next_step = "mfa".to_string();
                        available_methods = Some(methods);
                    }
                }
            }
        }
    }

    let state_id = create_state(
        &state.redis,
        principal_id,
        &request.email,
        &next_step,
        request.device_fingerprint,
        None,
    )
    .await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(IdentifierResult {
            next_step,
            state_token: state_id,
            available_methods,
        }),
    )
        .into_response())
}
