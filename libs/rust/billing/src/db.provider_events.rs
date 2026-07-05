use sqlx::PgPool;
use uuid::Uuid;

use crate::provider::{ProviderCode, ProviderWebhookEvent};
use crate::types::ProviderEventRecord;

pub async fn record_provider_event(
    db: &PgPool,
    input: &ProviderWebhookEvent,
    tenant_id: Option<Uuid>,
) -> Result<ProviderEventRecord, sqlx::Error> {
    sqlx::query_as::<_, ProviderEventRecord>(
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
    .bind(input.provider.as_str())
    .bind(&input.provider_event_id)
    .bind(&input.event_type)
    .bind(input.signature_valid)
    .bind(&input.payload_hash)
    .bind(sqlx::types::Json(&input.payload_summary))
    .bind(&input.raw_retention_class)
    .fetch_one(db)
    .await
}

pub async fn mark_provider_event_replayed(
    db: &PgPool,
    provider: ProviderCode,
    provider_event_id: &str,
) -> Result<Option<ProviderEventRecord>, sqlx::Error> {
    sqlx::query_as::<_, ProviderEventRecord>(
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
    .bind(provider.as_str())
    .bind(provider_event_id)
    .fetch_optional(db)
    .await
}

pub async fn mark_provider_event_processed(
    db: &PgPool,
    provider: ProviderCode,
    provider_event_id: &str,
    tenant_id: Option<Uuid>,
) -> Result<ProviderEventRecord, sqlx::Error> {
    sqlx::query_as::<_, ProviderEventRecord>(
        r#"
        UPDATE billing_provider_events
        SET status = 'processed',
            tenant_id = COALESCE($3, tenant_id),
            processed_at = NOW(),
            updated_at = NOW()
        WHERE provider = $1::billing_provider
          AND provider_event_id = $2
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
    .bind(provider.as_str())
    .bind(provider_event_id)
    .bind(tenant_id)
    .fetch_one(db)
    .await
}

pub async fn mark_provider_event_failed(
    db: &PgPool,
    provider: ProviderCode,
    provider_event_id: &str,
    error_code: &str,
    error_message: &str,
) -> Result<ProviderEventRecord, sqlx::Error> {
    sqlx::query_as::<_, ProviderEventRecord>(
        r#"
        UPDATE billing_provider_events
        SET status = 'failed',
            payload_summary = payload_summary || jsonb_build_object(
              'processing_error', jsonb_build_object(
                'code', $3,
                'message', $4
              )
            ),
            updated_at = NOW()
        WHERE provider = $1::billing_provider
          AND provider_event_id = $2
          AND status <> 'processed'
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
    .bind(provider.as_str())
    .bind(provider_event_id)
    .bind(error_code)
    .bind(error_message)
    .fetch_one(db)
    .await
}
