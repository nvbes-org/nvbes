use super::super::stripe::{parse_stripe_event, verify_stripe_signature};
use super::super::types::BillingWebhookResponse;
use crate::http::error::AppError;
use nvbes_billing::hex_encode;
use nvbes_core::config::AppConfig;
use sha2::{Digest, Sha256};

pub async fn handle_webhook(
    db: &sqlx::PgPool,
    config: &AppConfig,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<BillingWebhookResponse, AppError> {
    verify_stripe_signature(config, signature_header, payload)?;
    let event = parse_stripe_event(payload)?;
    let payload_hash = hex_encode(&Sha256::digest(payload));
    let payload_json: serde_json::Value = serde_json::from_slice(payload).map_err(|_| {
        AppError::bad_request(
            "invalid_webhook_payload",
            "Webhook payload is not valid JSON.",
        )
    })?;

    let mut tx = db.begin().await?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO billing_webhook_events (
          provider,
          provider_event_id,
          status,
          signature_valid,
          payload
        )
        VALUES ('stripe', $1, 'received', TRUE, $2)
        ON CONFLICT (provider_event_id) DO NOTHING
        "#,
    )
    .bind(&event.id)
    .bind(sqlx::types::Json(serde_json::json!({
        "hash": payload_hash,
        "event": payload_json,
    })))
    .execute(tx.as_mut())
    .await?;

    if inserted.rows_affected() == 0 {
        tx.commit().await?;
        return Ok(BillingWebhookResponse {
            provider_event_id: event.id,
            status: "duplicate".to_string(),
        });
    }

    let processed = super::processors::process_stripe_event(&mut tx, &event).await;
    match processed {
        Ok(()) => {
            sqlx::query(
                r#"
                UPDATE billing_webhook_events
                SET status = 'processed',
                    processed_at = NOW()
                WHERE provider_event_id = $1
                "#,
            )
            .bind(&event.id)
            .execute(tx.as_mut())
            .await?;
            tx.commit().await?;
            Ok(BillingWebhookResponse {
                provider_event_id: event.id,
                status: "processed".to_string(),
            })
        }
        Err(error) => {
            sqlx::query(
                r#"
                UPDATE billing_webhook_events
                SET status = 'failed',
                    processed_at = NOW()
                WHERE provider_event_id = $1
                "#,
            )
            .bind(&event.id)
            .execute(tx.as_mut())
            .await?;
            tx.commit().await?;
            Err(error)
        }
    }
}
