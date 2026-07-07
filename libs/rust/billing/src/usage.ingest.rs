use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::UsageEvent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingUsageIngestResponse {
    pub accepted: bool,
}

#[derive(Debug, Error)]
pub enum BillingUsageIngestError {
    #[error("{message}")]
    Validation {
        code: &'static str,
        message: &'static str,
    },
    #[error("usage ingestion database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("usage ingestion serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl BillingUsageIngestError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation { code, .. } => code,
            Self::Database(_) => "database_error",
            Self::Serialization(_) => "json_error",
        }
    }
}

pub async fn ingest_usage_event(
    db: &sqlx::PgPool,
    event: UsageEvent,
) -> Result<BillingUsageIngestResponse, BillingUsageIngestError> {
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

pub fn validate_usage_event(event: &UsageEvent) -> Result<(), BillingUsageIngestError> {
    if event.meter_code.trim().is_empty() {
        return Err(BillingUsageIngestError::Validation {
            code: "usage_meter_required",
            message: "Usage events require a meter code.",
        });
    }
    if event.quantity < 0 {
        return Err(BillingUsageIngestError::Validation {
            code: "usage_quantity_invalid",
            message: "Usage event quantity must be non-negative.",
        });
    }
    if event.source.trim().is_empty() || event.idempotency_key.trim().is_empty() {
        return Err(BillingUsageIngestError::Validation {
            code: "usage_idempotency_required",
            message: "Usage events require source and idempotency key.",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use super::*;

    fn usage_event(idempotency_key: &str, quantity: i64) -> UsageEvent {
        UsageEvent {
            tenant_id: Uuid::nil(),
            workspace_id: None,
            meter_code: "storage_gb_month".to_string(),
            quantity,
            unit: "gb_month".to_string(),
            occurred_at: DateTime::<Utc>::from_timestamp(1_735_689_600, 0).unwrap(),
            source: "cloud-service".to_string(),
            idempotency_key: idempotency_key.to_string(),
        }
    }

    #[test]
    fn usage_event_validation_requires_meter_quantity_and_idempotency() {
        assert!(validate_usage_event(&usage_event("snapshot-1", 1)).is_ok());

        let mut invalid = usage_event("snapshot-1", -1);
        assert_eq!(
            validate_usage_event(&invalid)
                .expect_err("negative usage should fail")
                .code(),
            "usage_quantity_invalid"
        );

        invalid = usage_event("", 1);
        assert_eq!(
            validate_usage_event(&invalid)
                .expect_err("missing idempotency should fail")
                .code(),
            "usage_idempotency_required"
        );
    }
}
