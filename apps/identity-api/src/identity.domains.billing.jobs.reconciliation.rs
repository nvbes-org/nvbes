use chrono::{DateTime, Duration, Utc};
use nvbes_billing::reconciliation::{
    ReconciliationDifference, ReconciliationDifferenceType, detect_amount_mismatch,
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::jobs_reconciliation_sources::{
    captured_payments_without_provider_evidence, duplicate_provider_captures, insert_difference,
    unbalanced_ledger_sources,
};
use crate::http::error::AppError;

pub fn reconcile_amounts(
    provider_amount_minor: i64,
    internal_amount_minor: i64,
) -> Option<ReconciliationDifference> {
    detect_amount_mismatch(provider_amount_minor, internal_amount_minor)
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingReconciliationRunResult {
    pub run_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub differences_created: u64,
}

pub async fn run_ledger_reconciliation(
    db: &PgPool,
    period_end: DateTime<Utc>,
) -> Result<BillingReconciliationRunResult, AppError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_difference_type_codes_match_schema_values() {
        assert_eq!(
            difference_type_code(ReconciliationDifferenceType::MissingProviderEvent),
            "missing_provider_event"
        );
        assert_eq!(
            difference_type_code(ReconciliationDifferenceType::UnbalancedLedger),
            "unbalanced_ledger"
        );
        assert_eq!(
            difference_type_code(ReconciliationDifferenceType::DuplicateCapture),
            "duplicate_capture"
        );
    }
}
