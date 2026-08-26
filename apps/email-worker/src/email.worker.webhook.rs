use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use serde::Serialize;
use std::time::Instant;

use crate::{
    email_metrics, error_reporting, state::EmailWorkerState, webhook_db, webhook_verify::SnsMessage,
};

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

#[tracing::instrument(
    name = "email.provider_webhook",
    skip_all,
    fields(
        http.request.method = "POST",
        http.route = "/webhooks/scaleway/topics-and-events",
        email.provider = "scaleway",
        email.event_type = tracing::field::Empty,
        otel.status_code = tracing::field::Empty,
    )
)]
async fn handle_scaleway_webhook(
    State(state): State<EmailWorkerState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let started_at = Instant::now();
    if !is_sns_content_type(&headers) {
        email_metrics::webhook("sns", "invalid_content_type", started_at.elapsed());
        return response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "invalid_content_type");
    }
    let Some(verifier) = state.webhook_verifier.as_ref() else {
        email_metrics::webhook("sns", "disabled", started_at.elapsed());
        return response(StatusCode::SERVICE_UNAVAILABLE, "webhook_disabled");
    };
    let message = match verifier.verify(&body).await {
        Ok(message) => message,
        Err(error) => {
            email_metrics::webhook_signature_failure();
            let outcome = error.outcome();
            email_metrics::webhook("sns", outcome, started_at.elapsed());
            tracing::warn!(verification_outcome = outcome, error = ?error, "Scaleway webhook signature validation failed");
            return response(StatusCode::UNAUTHORIZED, "invalid_signature");
        }
    };

    let event_type = webhook_event_type(&message);
    tracing::Span::current().record("email.event_type", event_type.as_str());
    let result = match message.message_type.as_str() {
        "SubscriptionConfirmation" => confirm_subscription(&state, &message).await,
        "Notification" => apply_notification(&state, &message).await,
        _ => return response(StatusCode::BAD_REQUEST, "unsupported_message"),
    };
    match result {
        Ok((status, outcome)) => {
            email_metrics::webhook(&event_type, outcome, started_at.elapsed());
            response(StatusCode::OK, status)
        }
        Err(error) => {
            tracing::Span::current().record("otel.status_code", "ERROR");
            email_metrics::webhook(&event_type, "processing_failed", started_at.elapsed());
            error_reporting::capture_operation_message(
                &state.config,
                "webhook_processing",
                "Scaleway webhook processing failed",
            );
            tracing::error!(sns_message_id = %message.message_id, error = ?error, "Scaleway webhook procedure failed");
            response(StatusCode::SERVICE_UNAVAILABLE, "processing_failed")
        }
    }
}

async fn confirm_subscription(
    state: &EmailWorkerState,
    message: &SnsMessage,
) -> anyhow::Result<(&'static str, &'static str)> {
    let verifier = state
        .webhook_verifier
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("webhook verifier is unavailable"))?;
    let confirmation_url = verifier.confirmation_url(message)?;
    verifier.confirm(confirmation_url).await?;
    let inserted =
        webhook_db::record_subscription_confirmation(&state.db, &message.message_id).await?;
    Ok(if inserted {
        ("confirmed", "confirmed")
    } else {
        ("duplicate", "duplicate")
    })
}

async fn apply_notification(
    state: &EmailWorkerState,
    message: &SnsMessage,
) -> anyhow::Result<(&'static str, &'static str)> {
    let event: webhook_db::TemWebhookEvent = serde_json::from_str(&message.message)?;
    if event.id.is_empty() || event.email_id.is_empty() || event.event_type.is_empty() {
        anyhow::bail!("TEM webhook event is incomplete");
    }
    let result = webhook_db::apply_notification(&state.db, &message.message_id, &event).await?;
    Ok(if result.inserted {
        ("processed", result.processing_result)
    } else {
        ("duplicate", result.processing_result)
    })
}

fn webhook_event_type(message: &SnsMessage) -> String {
    if message.message_type == "SubscriptionConfirmation" {
        return "subscription_confirmation".to_string();
    }
    serde_json::from_str::<serde_json::Value>(&message.message)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn is_sns_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "application/json" | "text/plain"
            )
        })
}

fn response(status: StatusCode, label: &'static str) -> (StatusCode, Json<WebhookResponse>) {
    (status, Json(WebhookResponse { status: label }))
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{SnsMessage, is_sns_content_type, response, webhook_event_type};

    #[test]
    fn webhook_accepts_only_sns_envelope_content_types() {
        let mut headers = HeaderMap::new();
        assert!(!is_sns_content_type(&headers));
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        assert!(is_sns_content_type(&headers));
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        assert!(is_sns_content_type(&headers));
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        assert!(!is_sns_content_type(&headers));
    }

    #[test]
    fn webhook_labels_are_bounded_for_confirmations_notifications_and_invalid_json() {
        let mut message = SnsMessage {
            message_type: "SubscriptionConfirmation".to_string(),
            message_id: "id".to_string(),
            topic_arn: "topic".to_string(),
            message: "{}".to_string(),
            timestamp: "2026-08-05T00:00:00Z".to_string(),
            signature_version: "1".to_string(),
            signature: "signature".to_string(),
            signing_cert_url: "https://example.test/cert.pem".to_string(),
            subject: None,
            token: None,
            subscribe_url: None,
        };
        assert_eq!(webhook_event_type(&message), "subscription_confirmation");
        message.message_type = "Notification".to_string();
        message.message = r#"{"type":"email_delivered"}"#.to_string();
        assert_eq!(webhook_event_type(&message), "email_delivered");
        message.message = "invalid".to_string();
        assert_eq!(webhook_event_type(&message), "unknown");
        let value = response(axum::http::StatusCode::BAD_REQUEST, "invalid");
        assert_eq!(value.0, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(value.1.0.status, "invalid");
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.webhook.tests.rs"]
mod database_tests;
