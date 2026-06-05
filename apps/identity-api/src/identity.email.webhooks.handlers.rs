use axum::{Json, body::Bytes, extract::State, http::HeaderMap, routing::post};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::warn;

use crate::app::AppState;
use crate::http::error::AppError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize)]
pub struct EmailWebhookResponse {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ScalewayEmailEvent {
    pub id: String,
    pub email_id: String,
    pub event_type: String,
    pub email: String,
    pub timestamp: String,
    #[serde(default)]
    pub message: Option<ScalewayMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ScalewayMessage {
    pub subject: Option<String>,
    pub message_id: Option<String>,
}

pub fn webhook_router(_state: &AppState) -> axum::Router<AppState> {
    axum::Router::new().route(
        "/webhooks/email/scaleway",
        post(handle_scaleway_email_webhook),
    )
}

async fn handle_scaleway_email_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<EmailWebhookResponse>, AppError> {
    let signature = headers
        .get("x-scaleway-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::unauthorized("missing_signature", "Missing X-Scaleway-Signature header")
        })?;

    let webhook_secret = state
        .config
        .scw_tem_webhook_secret
        .as_deref()
        .ok_or_else(|| {
            AppError::conflict(
                "webhook_not_configured",
                "SCW_TEM_WEBHOOK_SECRET must be configured before receiving email webhooks.",
            )
        })?;

    verify_scaleway_signature(webhook_secret, &body, signature)?;

    let event: ScalewayEmailEvent = serde_json::from_slice(&body)?;

    let mut tx = state.db.begin().await?;
    super::super::db::record_email_event_tx(&mut tx, &event).await?;

    tx.commit().await?;

    if is_actionable_event(&event.event_type) {
        super::super::jobs::enqueue_email_event_job_tx(&state.redis, &event).await?;
    }

    Ok(Json(EmailWebhookResponse {
        status: "accepted".to_string(),
    }))
}

fn verify_scaleway_signature(
    secret: &str,
    body: &[u8],
    signature_header: &str,
) -> Result<(), AppError> {
    let expected_prefix = "sha256=";
    let hex_digest = signature_header
        .strip_prefix(expected_prefix)
        .ok_or_else(|| {
            AppError::unauthorized(
                "invalid_signature_format",
                "X-Scaleway-Signature must start with 'sha256='",
            )
        })?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::internal("hmac_init_error", "Failed to initialize HMAC"))?;
    mac.update(body);
    let computed = hex::encode(mac.finalize().into_bytes());

    if computed != hex_digest {
        warn!(
            expected = %hex_digest,
            computed = %computed,
            "Scaleway webhook signature mismatch"
        );
        return Err(AppError::unauthorized(
            "invalid_signature",
            "Scaleway webhook signature verification failed.",
        ));
    }

    Ok(())
}

fn is_actionable_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "email_mailbox_not_found"
            | "email_spam"
            | "email_bounced"
            | "email_blacklisted"
            | "email_unsubscribed"
            | "email_delivered"
            | "email_dropped"
    )
}
