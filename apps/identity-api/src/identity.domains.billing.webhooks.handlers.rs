use crate::http::error::AppError;
use nvbes_core::config::AppConfig;
use nvbes_observability::metrics::HttpMetrics;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use super::super::{jobs, stripe, types};
use super::logic::{
    WebhookRetryDecision, classify_webhook_retry, resolve_webhook_workspace_id,
    summarize_webhook_payload,
};
use stripe::parse_stripe_event;
use types::BillingWebhookResponse;

pub async fn handle_webhook(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    observability: &HttpMetrics,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<BillingWebhookResponse, AppError> {
    let started_at = std::time::Instant::now();
    let webhook_secret = config.stripe_webhook_secret.as_deref().ok_or_else(|| {
        AppError::conflict(
            "stripe_webhook_not_configured",
            "NVBES_STRIPE_WEBHOOK_SECRET must be configured before receiving webhooks.",
        )
    })?;
    nvbes_billing::verify_stripe_signature(webhook_secret, signature_header, payload).map_err(
        |reason| match reason {
            "missing_stripe_signature" => AppError::unauthorized(
                "missing_stripe_signature",
                "Missing Stripe-Signature header.",
            ),
            "stale_stripe_signature" => AppError::unauthorized(
                "stale_stripe_signature",
                "Stripe webhook signature timestamp is outside the allowed tolerance.",
            ),
            "invalid_stripe_signature_timestamp" => AppError::unauthorized(
                "invalid_stripe_signature_timestamp",
                "Stripe signature timestamp is invalid.",
            ),
            _ => AppError::unauthorized(
                "invalid_stripe_signature",
                "Stripe webhook signature verification failed.",
            ),
        },
    )?;
    let event = parse_stripe_event(payload)?;
    let payload_hash = nvbes_billing::hex_encode(&Sha256::digest(payload));
    let payload_json: Value = serde_json::from_slice(payload).map_err(|_| {
        AppError::bad_request(
            "invalid_webhook_payload",
            "Webhook payload is not valid JSON.",
        )
    })?;
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

    match classify_webhook_retry(existing_status.as_deref()) {
        WebhookRetryDecision::Duplicate => {
            tx.commit().await?;
            observability.record_billing_webhook(
                "stripe",
                &event.event_type,
                "duplicate",
                started_at.elapsed(),
            );
            return Ok(BillingWebhookResponse {
                provider_event_id: event.id,
                status: "duplicate".to_string(),
            });
        }
        WebhookRetryDecision::ReplayFailed => {
            sqlx::query(
                r#"
                UPDATE billing_webhook_events
                SET workspace_id = $2,
                    status = 'received',
                    signature_valid = TRUE,
                    payload_summary = $3,
                    processed_at = NULL
                WHERE provider_event_id = $1
                "#,
            )
            .bind(&event.id)
            .bind(workspace_id)
            .bind(sqlx::types::Json(payload_record.clone()))
            .execute(tx.as_mut())
            .await?;
            observability.record_billing_webhook(
                "stripe",
                &event.event_type,
                "accepted",
                started_at.elapsed(),
            );
        }
        WebhookRetryDecision::Insert => {
            let inserted = sqlx::query(
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
                ON CONFLICT (provider_event_id) DO NOTHING
                "#,
            )
            .bind(&event.id)
            .bind(workspace_id)
            .bind(sqlx::types::Json(payload_record.clone()))
            .execute(tx.as_mut())
            .await?;
            if inserted.rows_affected() == 0 {
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

                match classify_webhook_retry(existing_status.as_deref()) {
                    WebhookRetryDecision::Duplicate => {
                        tx.commit().await?;
                        observability.record_billing_webhook(
                            "stripe",
                            &event.event_type,
                            "duplicate",
                            started_at.elapsed(),
                        );
                        return Ok(BillingWebhookResponse {
                            provider_event_id: event.id,
                            status: "duplicate".to_string(),
                        });
                    }
                    WebhookRetryDecision::ReplayFailed => {
                        sqlx::query(
                            r#"
                            UPDATE billing_webhook_events
                            SET workspace_id = $2,
                                status = 'received',
                                signature_valid = TRUE,
                                payload_summary = $3,
                                processed_at = NULL
                            WHERE provider_event_id = $1
                            "#,
                        )
                        .bind(&event.id)
                        .bind(workspace_id)
                        .bind(sqlx::types::Json(payload_record.clone()))
                        .execute(tx.as_mut())
                        .await?;
                        observability.record_billing_webhook(
                            "stripe",
                            &event.event_type,
                            "accepted",
                            started_at.elapsed(),
                        );
                    }
                    WebhookRetryDecision::Insert => {
                        observability.record_billing_webhook(
                            "stripe",
                            &event.event_type,
                            "received",
                            started_at.elapsed(),
                        );
                    }
                }
            } else {
                observability.record_billing_webhook(
                    "stripe",
                    &event.event_type,
                    "received",
                    started_at.elapsed(),
                );
            }
        }
    }

    tx.commit().await?;
    jobs::enqueue_stripe_webhook_job_tx(redis, payload_json, &event.id).await?;
    observability.record_billing_webhook(
        "stripe",
        &event.event_type,
        "accepted",
        started_at.elapsed(),
    );
    Ok(BillingWebhookResponse {
        provider_event_id: event.id,
        status: "accepted".to_string(),
    })
}
