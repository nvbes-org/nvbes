use sqlx::PgPool;
use uuid::Uuid;

use super::super::types::ProviderEventRecord;
use crate::http::error::AppError;

pub async fn record_provider_event(
    db: &PgPool,
    input: &nvbes_billing::provider::ProviderWebhookEvent,
    tenant_id: Option<Uuid>,
) -> Result<ProviderEventRecord, AppError> {
    let provider = match input.provider {
        nvbes_billing::provider::ProviderCode::Stripe => "stripe",
        nvbes_billing::provider::ProviderCode::Mollie => "mollie",
    };

    let record = sqlx::query_as::<_, ProviderEventRecord>(
        r#"
        INSERT INTO billing_provider_events (
          tenant_id,
          provider,
          provider_event_id,
          event_type,
          status,
          signature_valid,
          payload_hash,
          payload_summary,
          raw_retention_class
        )
        VALUES ($1, $2::billing_provider, $3, $4, 'verified', $5, $6, $7, $8)
        ON CONFLICT (provider, provider_event_id) DO UPDATE
        SET status = CASE
              WHEN billing_provider_events.status IN ('failed', 'rejected') THEN 'verified'::billing_provider_event_status
              ELSE billing_provider_events.status
            END,
            signature_valid = EXCLUDED.signature_valid,
            payload_summary = EXCLUDED.payload_summary,
            updated_at = NOW()
        RETURNING
          id,
          tenant_id,
          provider::text AS provider,
          provider_event_id,
          event_type,
          status::text AS status,
          signature_valid,
          payload_summary
        "#,
    )
    .bind(tenant_id)
    .bind(provider)
    .bind(&input.provider_event_id)
    .bind(&input.event_type)
    .bind(input.signature_valid)
    .bind(&input.payload_hash)
    .bind(sqlx::types::Json(&input.payload_summary))
    .bind(&input.raw_retention_class)
    .fetch_one(db)
    .await?;

    Ok(record)
}

pub async fn mark_provider_event_replayed(
    db: &PgPool,
    provider: &str,
    provider_event_id: &str,
) -> Result<ProviderEventRecord, AppError> {
    let record = sqlx::query_as::<_, ProviderEventRecord>(
        r#"
        UPDATE billing_provider_events
        SET status = 'replayed',
            updated_at = NOW()
        WHERE provider = $1::billing_provider
          AND provider_event_id = $2
          AND status IN ('failed', 'rejected', 'verified')
        RETURNING
          id,
          tenant_id,
          provider::text AS provider,
          provider_event_id,
          event_type,
          status::text AS status,
          signature_valid,
          payload_summary
        "#,
    )
    .bind(provider)
    .bind(provider_event_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "provider_event_not_replayable",
            "Provider event is missing or not in a replayable state.",
        )
    })?;

    Ok(record)
}
