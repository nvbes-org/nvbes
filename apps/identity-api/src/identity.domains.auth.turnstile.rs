use crate::http::error::AppError;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize)]
struct TurnstileRequest<'a> {
    secret: &'a str,
    response: &'a str,
    remoteip: Option<&'a str>,
    idempotency_key: Option<&'a str>,
}

#[derive(Deserialize, Debug)]
pub struct TurnstileResponse {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
    pub challenge_ts: Option<String>,
    pub hostname: Option<String>,
    pub action: Option<String>,
    pub cdata: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VerifyOptions<'a> {
    pub ip: Option<&'a str>,
    pub expected_action: Option<&'a str>,
    pub expected_hostname: Option<&'a str>,
    pub idempotency_key: Option<&'a str>,
}

#[instrument(skip(secret, token, options))]
pub async fn verify_token(
    secret: &str,
    token: &str,
    options: VerifyOptions<'_>,
) -> Result<(), AppError> {
    if token.trim().is_empty() {
        return Err(AppError::bad_request(
            "missing_turnstile_token",
            "Turnstile token cannot be empty",
        ));
    }

    let client = nvbes_core::security::pinned_http_client();

    let request = client
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .json(&TurnstileRequest {
            secret,
            response: token,
            remoteip: options.ip,
            idempotency_key: options.idempotency_key,
        });
    let res = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to contact Cloudflare Turnstile API: {:?}", e);
            AppError::internal(
                "turnstile_network_error",
                "Failed to communicate with the bot protection service",
            )
        })?;

    let parsed: TurnstileResponse = res.json().await.map_err(|e| {
        tracing::error!("Failed to parse Turnstile API response: {:?}", e);
        AppError::internal(
            "turnstile_parse_error",
            "Invalid response from the bot protection service",
        )
    })?;

    if !parsed.success {
        let errors = parsed.error_codes.clone().unwrap_or_default();
        tracing::warn!("Turnstile verification failed. Error codes: {:?}", errors);

        if errors.contains(&"timeout-or-duplicate".to_string()) {
            return Err(AppError::forbidden(
                "turnstile_timeout_or_duplicate",
                "The bot protection token has expired or has already been used.",
            ));
        }

        if errors.contains(&"invalid-input-response".to_string()) {
            return Err(AppError::forbidden(
                "turnstile_invalid_token",
                "The bot protection token is invalid or malformed.",
            ));
        }

        return Err(AppError::forbidden(
            "bot_detected",
            "Security challenge verification failed. Please try again.",
        ));
    }

    // 1. Validate hostname to prevent domain-spoofing/host-spoofing replay attacks
    if let Some(expected_host) = options.expected_hostname {
        if let Some(actual_host) = &parsed.hostname {
            // Strip port if present in expected or actual hostnames
            let clean_expected = expected_host.split(':').next().unwrap_or(expected_host);
            let clean_actual = actual_host.split(':').next().unwrap_or(actual_host);

            if clean_expected != clean_actual
                && clean_actual != "localhost"
                && clean_actual != "127.0.0.1"
            {
                tracing::error!(
                    "Turnstile domain spoofing detected! Expected: {}, Got: {}",
                    clean_expected,
                    clean_actual
                );
                return Err(AppError::forbidden(
                    "turnstile_domain_mismatch",
                    "Security challenge was completed on an invalid domain.",
                ));
            }
        } else {
            tracing::warn!("Turnstile response missing hostname for validation");
            return Err(AppError::forbidden(
                "turnstile_domain_missing",
                "Domain verification failed on bot protection challenge.",
            ));
        }
    }

    // 2. Validate action to prevent token reuse/cross-action replay
    if let Some(expected_act) = options.expected_action {
        if let Some(actual_act) = &parsed.action {
            if expected_act != actual_act {
                tracing::error!(
                    "Turnstile action token reuse detected! Expected: {}, Got: {}",
                    expected_act,
                    actual_act
                );
                return Err(AppError::forbidden(
                    "turnstile_action_mismatch",
                    "Security challenge token is not valid for this action.",
                ));
            }
        } else {
            tracing::warn!("Turnstile response missing action for validation");
            return Err(AppError::forbidden(
                "turnstile_action_missing",
                "Action verification failed on bot protection challenge.",
            ));
        }
    }

    // 3. Validate token age via challenge_ts (Cloudflare tokens live 5 mins max)
    if let Some(ts_str) = &parsed.challenge_ts {
        if let Ok(challenge_time) = chrono::DateTime::parse_from_rfc3339(ts_str) {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(challenge_time.with_timezone(&chrono::Utc));
            if duration.num_seconds() > 300 {
                tracing::warn!(
                    "Turnstile token is too old: {} seconds",
                    duration.num_seconds()
                );
                return Err(AppError::forbidden(
                    "turnstile_token_expired",
                    "Bot protection token has expired.",
                ));
            }
        }
    }

    Ok(())
}
