use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use serde::Serialize;

use crate::{state::EmailWorkerState, webhook_db, webhook_verify::SnsMessage};

const MAX_WEBHOOK_BODY_BYTES: usize = 256 * 1024;

#[derive(Serialize)]
struct WebhookResponse {
    status: &'static str,
}

pub fn router(state: EmailWorkerState) -> Router {
    Router::new()
        .route(
            "/webhooks/scaleway/topics-and-events",
            post(handle_scaleway_webhook),
        )
        .layer(DefaultBodyLimit::max(MAX_WEBHOOK_BODY_BYTES))
        .with_state(state)
}

async fn handle_scaleway_webhook(
    State(state): State<EmailWorkerState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if !is_json(&headers) {
        return response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "invalid_content_type");
    }
    let Some(verifier) = state.webhook_verifier.as_ref() else {
        return response(StatusCode::SERVICE_UNAVAILABLE, "webhook_disabled");
    };
    let message = match verifier.verify(&body).await {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(error = ?error, "Scaleway webhook signature validation failed");
            return response(StatusCode::UNAUTHORIZED, "invalid_signature");
        }
    };

    let result = match message.message_type.as_str() {
        "SubscriptionConfirmation" => confirm_subscription(&state, &message).await,
        "Notification" => apply_notification(&state, &message).await,
        _ => return response(StatusCode::BAD_REQUEST, "unsupported_message"),
    };
    match result {
        Ok(status) => response(StatusCode::OK, status),
        Err(error) => {
            tracing::error!(sns_message_id = %message.message_id, error = ?error, "Scaleway webhook procedure failed");
            response(StatusCode::SERVICE_UNAVAILABLE, "processing_failed")
        }
    }
}

async fn confirm_subscription(
    state: &EmailWorkerState,
    message: &SnsMessage,
) -> anyhow::Result<&'static str> {
    let verifier = state
        .webhook_verifier
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("webhook verifier is unavailable"))?;
    let confirmation_url = verifier.confirmation_url(message)?;
    verifier.confirm(confirmation_url).await?;
    let inserted =
        webhook_db::record_subscription_confirmation(&state.db, &message.message_id).await?;
    Ok(if inserted { "confirmed" } else { "duplicate" })
}

async fn apply_notification(
    state: &EmailWorkerState,
    message: &SnsMessage,
) -> anyhow::Result<&'static str> {
    let event: webhook_db::TemWebhookEvent = serde_json::from_str(&message.message)?;
    if event.id.is_empty() || event.email_id.is_empty() || event.event_type.is_empty() {
        anyhow::bail!("TEM webhook event is incomplete");
    }
    let inserted = webhook_db::apply_notification(&state.db, &message.message_id, &event).await?;
    Ok(if inserted { "processed" } else { "duplicate" })
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
}

fn response(status: StatusCode, label: &'static str) -> (StatusCode, Json<WebhookResponse>) {
    (status, Json(WebhookResponse { status: label }))
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::is_json;

    #[test]
    fn webhook_requires_json_content_type() {
        let mut headers = HeaderMap::new();
        assert!(!is_json(&headers));
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        assert!(is_json(&headers));
    }
}
