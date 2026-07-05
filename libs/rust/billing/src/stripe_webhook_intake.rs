use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    StripeWebhookEvent, hex_encode, metadata_workspace_id, parse_stripe_event, parse_uuid,
    verify_stripe_signature,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingWebhookIntakeResponse {
    pub provider: String,
    pub provider_event_id: String,
    pub event_type: String,
    pub status: String,
}

#[derive(Debug, Error)]
pub enum BillingWebhookIntakeError {
    #[error("Stripe webhook secret is not configured")]
    MissingStripeWebhookSecret,
    #[error("Stripe webhook signature verification failed: {0}")]
    StripeSignature(&'static str),
    #[error("Webhook payload is not valid JSON or missing required fields")]
    InvalidPayload,
    #[error("webhook intake database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Queue(#[from] crate::jobs::BillingJobQueueError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookRetryDecision {
    Insert,
    Duplicate,
    ReplayFailed,
}

pub async fn handle_stripe_webhook_intake(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    webhook_secret: Option<&str>,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<BillingWebhookIntakeResponse, BillingWebhookIntakeError> {
    let webhook_secret =
        webhook_secret.ok_or(BillingWebhookIntakeError::MissingStripeWebhookSecret)?;
    verify_stripe_signature(webhook_secret, signature_header, payload)
        .map_err(BillingWebhookIntakeError::StripeSignature)?;
    let event = parse_stripe_event(payload).ok_or(BillingWebhookIntakeError::InvalidPayload)?;
    let payload_hash = hex_encode(&Sha256::digest(payload));
    let payload_json: Value =
        serde_json::from_slice(payload).map_err(|_| BillingWebhookIntakeError::InvalidPayload)?;
    let payload_record = summarize_webhook_payload(&event, payload_hash, &payload_json);

    let mut tx = db.begin().await?;
    let workspace_id = resolve_webhook_workspace_id(&event.data_object);
    let existing_status = sqlx::query_scalar::<_, String>(
        r#"
        SELECT status
        FROM billing_webhook_events
        WHERE provider_event_id = $1
        FOR UPDATE
        "#,
    )
    .bind(&event.id)
    .fetch_optional(tx.as_mut())
    .await?;

    if classify_webhook_retry(existing_status.as_deref()) == WebhookRetryDecision::Duplicate {
        tx.commit().await?;
        return Ok(response(&event, "duplicate"));
    }

    upsert_received_webhook_event(tx.as_mut(), &event, workspace_id, payload_record).await?;
    tx.commit().await?;
    crate::jobs::enqueue_stripe_webhook_job(redis, payload_json, &event.id).await?;

    Ok(response(&event, "accepted"))
}

pub fn resolve_webhook_workspace_id(object: &Value) -> Option<Uuid> {
    metadata_workspace_id(object).or_else(|| {
        object
            .get("client_reference_id")
            .and_then(Value::as_str)
            .and_then(parse_uuid)
    })
}

pub fn summarize_webhook_payload(
    event: &StripeWebhookEvent,
    payload_hash: String,
    payload: &Value,
) -> Value {
    serde_json::json!({
        "hash": payload_hash,
        "event_id": event.id,
        "event_type": event.event_type,
        "object_id": payload.get("id").and_then(Value::as_str),
        "object_type": payload.get("object").and_then(Value::as_str),
        "livemode": payload.get("livemode").and_then(Value::as_bool),
        "created": payload.get("created").and_then(Value::as_i64),
    })
}

pub fn classify_webhook_retry(existing_status: Option<&str>) -> WebhookRetryDecision {
    match existing_status {
        Some("processed") => WebhookRetryDecision::Duplicate,
        Some("failed" | "received") => WebhookRetryDecision::ReplayFailed,
        _ => WebhookRetryDecision::Insert,
    }
}

async fn upsert_received_webhook_event(
    db: &mut sqlx::PgConnection,
    event: &StripeWebhookEvent,
    workspace_id: Option<Uuid>,
    payload_record: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO billing_webhook_events (
          provider,
          provider_event_id,
          workspace_id,
          status,
          signature_valid,
          payload_summary
        )
        VALUES ('stripe', $1, $2, 'received', TRUE, $3)
        ON CONFLICT (provider_event_id) DO UPDATE
        SET workspace_id = EXCLUDED.workspace_id,
            status = CASE
              WHEN billing_webhook_events.status IN ('failed', 'received') THEN 'received'::billing_webhook_status
              ELSE billing_webhook_events.status
            END,
            signature_valid = TRUE,
            payload_summary = EXCLUDED.payload_summary,
            processed_at = NULL
        "#,
    )
    .bind(&event.id)
    .bind(workspace_id)
    .bind(sqlx::types::Json(payload_record))
    .execute(db)
    .await?;
    Ok(())
}

fn response(event: &StripeWebhookEvent, status: &str) -> BillingWebhookIntakeResponse {
    BillingWebhookIntakeResponse {
        provider: "stripe".to_string(),
        provider_event_id: event.id.clone(),
        event_type: event.event_type.clone(),
        status: status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{WebhookRetryDecision, classify_webhook_retry};

    #[test]
    fn classify_webhook_retry_treats_processed_as_duplicate() {
        assert_eq!(
            classify_webhook_retry(Some("processed")),
            WebhookRetryDecision::Duplicate
        );
    }

    #[test]
    fn classify_webhook_retry_treats_failed_as_replayable() {
        assert_eq!(
            classify_webhook_retry(Some("failed")),
            WebhookRetryDecision::ReplayFailed
        );
    }

    #[test]
    fn classify_webhook_retry_treats_received_as_replayable() {
        assert_eq!(
            classify_webhook_retry(Some("received")),
            WebhookRetryDecision::ReplayFailed
        );
    }

    #[test]
    fn classify_webhook_retry_treats_missing_status_as_insert() {
        assert_eq!(classify_webhook_retry(None), WebhookRetryDecision::Insert);
    }
}
