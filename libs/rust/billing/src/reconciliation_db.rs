use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use thiserror::Error;
use uuid::Uuid;

use crate::reconciliation::ReconciliationDifferenceType;

#[derive(Debug, Error)]
pub enum BillingReconciliationError {
    #[error("billing reconciliation database error: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingReconciliationRunResult {
    pub run_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub differences_created: u64,
}

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

pub async fn run_ledger_reconciliation(
    db: &PgPool,
    period_end: DateTime<Utc>,
) -> Result<BillingReconciliationRunResult, BillingReconciliationError> {
    let period_start = period_end - Duration::days(1);
    let mut tx = db.begin().await?;
    let run_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_reconciliation_runs (
          provider, status, period_start, period_end, summary
        )
        VALUES (NULL, 'running', $1, $2, '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(period_start)
    .bind(period_end)
    .fetch_one(tx.as_mut())
    .await?;

    let unbalanced_sources =
        unbalanced_ledger_sources(tx.as_mut(), period_start, period_end).await?;
    let mut differences_created = 0_u64;
    for source in &unbalanced_sources {
        differences_created += insert_difference(
            tx.as_mut(),
            run_id,
            source.tenant_id,
            ReconciliationDifferenceType::UnbalancedLedger,
            "critical",
            serde_json::json!({
                "source_type": source.source_type,
                "source_id": source.source_id,
                "currency": source.currency,
                "balance_minor": source.balance_minor,
                "entry_count": source.entry_count,
            }),
        )
        .await?;
    }

    let payments_without_provider =
        captured_payments_without_provider_evidence(tx.as_mut(), period_start, period_end).await?;
    for payment in &payments_without_provider {
        differences_created += insert_difference(
            tx.as_mut(),
            run_id,
            payment.tenant_id,
            ReconciliationDifferenceType::MissingProviderEvent,
            "high",
            serde_json::json!({
                "payment_id": payment.payment_id,
                "provider": payment.provider,
                "amount_minor": payment.amount_minor,
                "currency": payment.currency,
            }),
        )
        .await?;
    }

    let duplicate_captures =
        duplicate_provider_captures(tx.as_mut(), period_start, period_end).await?;
    for capture in &duplicate_captures {
        differences_created += insert_difference(
            tx.as_mut(),
            run_id,
            capture.tenant_id,
            ReconciliationDifferenceType::DuplicateCapture,
            "critical",
            serde_json::json!({
                "provider": capture.provider,
                "provider_attempt_id": capture.provider_attempt_id,
                "payment_count": capture.payment_count,
                "total_amount_minor": capture.total_amount_minor,
            }),
        )
        .await?;
    }

    sqlx::query(
        r#"
        UPDATE billing_reconciliation_runs
        SET status = 'completed',
            summary = jsonb_build_object(
              'differences_created', $2,
              'checked', jsonb_build_object('ledger_balance', true)
            ),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(i64::try_from(differences_created).unwrap_or(i64::MAX))
    .execute(tx.as_mut())
    .await?;
    tx.commit().await?;

    Ok(BillingReconciliationRunResult {
        run_id,
        period_start,
        period_end,
        differences_created,
    })
}

pub async fn unbalanced_ledger_sources(
    db: &mut PgConnection,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<UnbalancedLedgerSource>, BillingReconciliationError> {
    Ok(sqlx::query_as::<_, UnbalancedLedgerSource>(
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
    .await?)
}

pub async fn captured_payments_without_provider_evidence(
    db: &mut PgConnection,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<CapturedPaymentWithoutProviderEvidence>, BillingReconciliationError> {
    Ok(sqlx::query_as::<_, CapturedPaymentWithoutProviderEvidence>(
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
    .await?)
}

pub async fn duplicate_provider_captures(
    db: &mut PgConnection,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<DuplicateProviderCapture>, BillingReconciliationError> {
    Ok(sqlx::query_as::<_, DuplicateProviderCapture>(
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
    .await?)
}

pub async fn insert_difference(
    db: &mut PgConnection,
    run_id: Uuid,
    tenant_id: Uuid,
    difference_type: ReconciliationDifferenceType,
    severity: &'static str,
    details: serde_json::Value,
) -> Result<u64, BillingReconciliationError> {
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
    .bind(difference_type_code(difference_type))
    .bind(severity)
    .bind(details)
    .execute(db)
    .await?;
    Ok(inserted.rows_affected())
}

pub fn difference_type_code(difference_type: ReconciliationDifferenceType) -> &'static str {
    match difference_type {
        ReconciliationDifferenceType::MissingProviderEvent => "missing_provider_event",
        ReconciliationDifferenceType::AmountMismatch => "amount_mismatch",
        ReconciliationDifferenceType::CurrencyMismatch => "currency_mismatch",
        ReconciliationDifferenceType::StatusMismatch => "status_mismatch",
        ReconciliationDifferenceType::DuplicateCapture => "duplicate_capture",
        ReconciliationDifferenceType::UnbalancedLedger => "unbalanced_ledger",
    }
}
