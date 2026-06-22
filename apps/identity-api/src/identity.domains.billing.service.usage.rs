use nvbes_billing::usage::UsageEvent;
use serde::{Deserialize, Serialize};

use crate::http::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingUsageIngestResponse {
    pub accepted: bool,
}

pub async fn ingest_usage_event(
    db: &sqlx::PgPool,
    event: UsageEvent,
) -> Result<BillingUsageIngestResponse, AppError> {
    validate_usage_event(&event)?;
    sqlx::query(
        r#"
        INSERT INTO billing_usage_events (
          tenant_id,
          workspace_id,
          meter_code,
          quantity,
          unit,
          occurred_at,
          source,
          idempotency_key,
          raw_event
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (tenant_id, source, idempotency_key) DO NOTHING
        "#,
    )
    .bind(event.tenant_id)
    .bind(event.workspace_id)
    .bind(&event.meter_code)
    .bind(event.quantity)
    .bind(&event.unit)
    .bind(event.occurred_at)
    .bind(&event.source)
    .bind(&event.idempotency_key)
    .bind(sqlx::types::Json(serde_json::to_value(&event)?))
    .execute(db)
    .await?;

    Ok(BillingUsageIngestResponse { accepted: true })
}

pub fn validate_usage_event(event: &UsageEvent) -> Result<(), AppError> {
    if event.meter_code.trim().is_empty() {
        return Err(AppError::bad_request(
            "usage_meter_required",
            "Usage events require a meter code.",
        ));
    }
    if event.quantity < 0 {
        return Err(AppError::bad_request(
            "usage_quantity_invalid",
            "Usage event quantity must be non-negative.",
        ));
    }
    if event.source.trim().is_empty() || event.idempotency_key.trim().is_empty() {
        return Err(AppError::bad_request(
            "usage_idempotency_required",
            "Usage events require source and idempotency key.",
        ));
    }
    Ok(())
}

#[cfg(test)]
fn usage_event_idempotency_scope(event: &UsageEvent) -> String {
    format!(
        "{}:{}:{}",
        event.tenant_id, event.source, event.idempotency_key
    )
}

#[cfg(test)]
fn test_usage_event() -> UsageEvent {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    UsageEvent {
        tenant_id: Uuid::nil(),
        workspace_id: Some(Uuid::nil()),
        meter_code: "storage_gb_month".to_string(),
        quantity: 1,
        unit: "gb_month".to_string(),
        occurred_at: DateTime::<Utc>::from_timestamp(1_782_000_000, 0).unwrap(),
        source: "drive-api".to_string(),
        idempotency_key: "snapshot-1".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_event_validation_requires_meter_quantity_and_idempotency() {
        assert!(validate_usage_event(&test_usage_event()).is_ok());

        let mut invalid = test_usage_event();
        invalid.quantity = -1;
        assert_eq!(
            validate_usage_event(&invalid)
                .expect_err("negative usage should fail")
                .code,
            "usage_quantity_invalid"
        );

        invalid = test_usage_event();
        invalid.idempotency_key.clear();
        assert_eq!(
            validate_usage_event(&invalid)
                .expect_err("missing idempotency should fail")
                .code,
            "usage_idempotency_required"
        );
    }

    #[test]
    fn usage_event_idempotency_scope_is_tenant_source_and_key() {
        assert_eq!(
            usage_event_idempotency_scope(&test_usage_event()),
            "00000000-0000-0000-0000-000000000000:drive-api:snapshot-1"
        );
    }
}
