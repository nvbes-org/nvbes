use chrono::{DateTime, Utc};
use nvbes_billing::reconciliation::ReconciliationDifferenceType;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, sqlx::FromRow)]
pub struct UnbalancedLedgerSource {
    pub tenant_id: Uuid,
    pub source_type: String,
    pub source_id: Uuid,
    pub currency: String,
    pub balance_minor: i64,
    pub entry_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CapturedPaymentWithoutProviderEvidence {
    pub tenant_id: Uuid,
    pub payment_id: Uuid,
    pub provider: String,
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DuplicateProviderCapture {
    pub tenant_id: Uuid,
    pub provider: String,
    pub provider_attempt_id: String,
    pub payment_count: i64,
    pub total_amount_minor: i64,
}

pub async fn unbalanced_ledger_sources(
    db: &mut PgConnection,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<UnbalancedLedgerSource>, AppError> {
    sqlx::query_as::<_, UnbalancedLedgerSource>(
        r#"
        SELECT
          tenant_id,
          source_type,
          source_id,
          currency::text AS currency,
          SUM(amount_minor)::bigint AS balance_minor,
          COUNT(*)::bigint AS entry_count
        FROM billing_ledger_entries
        WHERE occurred_at >= $1
          AND occurred_at < $2
        GROUP BY tenant_id, source_type, source_id, currency
        HAVING SUM(amount_minor) <> 0
        ORDER BY ABS(SUM(amount_minor)) DESC
        LIMIT 500
        "#,
    )
    .bind(period_start)
    .bind(period_end)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn captured_payments_without_provider_evidence(
    db: &mut PgConnection,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<CapturedPaymentWithoutProviderEvidence>, AppError> {
    sqlx::query_as::<_, CapturedPaymentWithoutProviderEvidence>(
        r#"
        SELECT
          p.tenant_id,
          p.id AS payment_id,
          p.provider::text AS provider,
          p.amount_minor,
          p.currency::text AS currency
        FROM billing_payments p
        LEFT JOIN billing_payment_attempts pa
          ON pa.payment_id = p.id
         AND pa.provider = p.provider
         AND pa.provider_attempt_id IS NOT NULL
         AND pa.status IN ('authorized', 'captured')
        LEFT JOIN billing_provider_mappings pm
          ON pm.provider = p.provider
         AND pm.local_entity_type = 'payment'
         AND pm.local_entity_id = p.id
         AND pm.provider_entity_type = 'payment'
         AND pm.status = 'active'
        WHERE p.status = 'captured'
          AND p.created_at >= $1
          AND p.created_at < $2
          AND pa.id IS NULL
          AND pm.id IS NULL
        ORDER BY p.created_at ASC, p.id ASC
        LIMIT 500
        "#,
    )
    .bind(period_start)
    .bind(period_end)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn duplicate_provider_captures(
    db: &mut PgConnection,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<DuplicateProviderCapture>, AppError> {
    sqlx::query_as::<_, DuplicateProviderCapture>(
        r#"
        SELECT
          p.tenant_id,
          pa.provider::text AS provider,
          pa.provider_attempt_id,
          COUNT(DISTINCT p.id)::bigint AS payment_count,
          SUM(p.amount_minor)::bigint AS total_amount_minor
        FROM billing_payment_attempts pa
        JOIN billing_payments p ON p.id = pa.payment_id
        WHERE pa.provider_attempt_id IS NOT NULL
          AND pa.status = 'captured'
          AND p.created_at >= $1
          AND p.created_at < $2
        GROUP BY p.tenant_id, pa.provider, pa.provider_attempt_id
        HAVING COUNT(DISTINCT p.id) > 1
        ORDER BY COUNT(DISTINCT p.id) DESC, pa.provider_attempt_id ASC
        LIMIT 500
        "#,
    )
    .bind(period_start)
    .bind(period_end)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn insert_difference(
    db: &mut PgConnection,
    run_id: Uuid,
    tenant_id: Uuid,
    difference_type: ReconciliationDifferenceType,
    severity: &'static str,
    details: serde_json::Value,
) -> Result<u64, AppError> {
    let inserted = sqlx::query(
        r#"
        INSERT INTO billing_reconciliation_differences (
          reconciliation_run_id, tenant_id, difference_type, severity, details
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(run_id)
    .bind(tenant_id)
    .bind(super::jobs_reconciliation::difference_type_code(
        difference_type,
    ))
    .bind(severity)
    .bind(details)
    .execute(db)
    .await?;
    Ok(inserted.rows_affected())
}
