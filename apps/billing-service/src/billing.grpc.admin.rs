use tonic::Status;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminCommandCenterBillingMetrics, AdminOperationsCenterSnapshot, RecentExportRun,
    RecentProviderFailure, RecentReconciliationDifference,
};

pub async fn command_center_metrics(
    db: &sqlx::PgPool,
) -> Result<AdminCommandCenterBillingMetrics, Status> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r#"
        SELECT
          (SELECT COUNT(*) FROM billing_kyc_profiles WHERE review_status = 'pending') AS pending_kyc_approval_count,
          (
            SELECT COUNT(*) FROM billing_provider_events
            WHERE status::text IN ('failed', 'rejected')
          ) AS provider_failure_count,
          (
            SELECT COUNT(*) FROM billing_invoices
            WHERE due_at < NOW() AND status::text IN ('issued', 'pro_forma')
          ) AS overdue_invoice_count,
          (
            SELECT COUNT(*) FROM billing_payments
            WHERE status::text IN ('failed', 'disputed')
          ) AS failed_payment_count
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(AdminCommandCenterBillingMetrics {
        pending_kyc_approval_count: row.0,
        provider_failure_count: row.1,
        overdue_invoice_count: row.2,
        failed_payment_count: row.3,
    })
}

pub async fn operations_center_snapshot(
    db: &sqlx::PgPool,
) -> Result<AdminOperationsCenterSnapshot, Status> {
    let metrics = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM billing_provider_events
            WHERE status::text IN ('failed', 'rejected')
          ) AS provider_event_failure_count,
          (
            SELECT COUNT(*) FROM billing_provider_events
            WHERE status::text IN ('received', 'verified')
          ) AS provider_event_backlog_count,
          (
            SELECT COUNT(*) FROM billing_export_runs
            WHERE status = 'pending'
          ) AS export_pending_count,
          (
            SELECT COUNT(*) FROM billing_export_runs
            WHERE status IN ('failed', 'error')
          ) AS export_failed_count,
          (
            SELECT COUNT(*) FROM billing_reconciliation_runs
            WHERE status = 'pending'
          ) AS reconciliation_pending_count,
          (
            SELECT COUNT(*) FROM billing_reconciliation_runs
            WHERE status IN ('failed', 'error')
          ) AS reconciliation_failed_count,
          (
            SELECT COUNT(*) FROM billing_reconciliation_differences
            WHERE resolved_at IS NULL
          ) AS unresolved_reconciliation_difference_count
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(AdminOperationsCenterSnapshot {
        provider_event_failure_count: metrics.0,
        provider_event_backlog_count: metrics.1,
        export_pending_count: metrics.2,
        export_failed_count: metrics.3,
        reconciliation_pending_count: metrics.4,
        reconciliation_failed_count: metrics.5,
        unresolved_reconciliation_difference_count: metrics.6,
        recent_provider_failures: recent_provider_failures(db).await?,
        recent_export_runs: recent_export_runs(db).await?,
        recent_reconciliation_differences: recent_reconciliation_differences(db).await?,
    })
}

async fn recent_provider_failures(
    db: &sqlx::PgPool,
) -> Result<Vec<RecentProviderFailure>, Status> {
    let rows = sqlx::query_as::<_, (uuid::Uuid, Option<uuid::Uuid>, Option<String>, String, String, String, String, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT bpe.id, bpe.tenant_id, t.name AS tenant_name, bpe.provider::text,
          bpe.provider_event_id, bpe.event_type, bpe.status::text, bpe.received_at
        FROM billing_provider_events bpe
        LEFT JOIN tenants t ON t.id = bpe.tenant_id
        WHERE bpe.status::text IN ('failed', 'rejected')
        ORDER BY bpe.received_at DESC, bpe.id DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| RecentProviderFailure {
            id: row.0.to_string(),
            tenant_id: row.1.map(|value| value.to_string()).unwrap_or_default(),
            tenant_name: row.2.unwrap_or_default(),
            provider: row.3,
            provider_event_id: row.4,
            event_type: row.5,
            status: row.6,
            received_at: row.7.to_rfc3339(),
        })
        .collect())
}

async fn recent_export_runs(db: &sqlx::PgPool) -> Result<Vec<RecentExportRun>, Status> {
    let rows = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<chrono::NaiveDate>, Option<chrono::NaiveDate>, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT id, export_type, status, period_start, period_end, created_at, updated_at
        FROM billing_export_runs
        ORDER BY created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| RecentExportRun {
            id: row.0.to_string(),
            export_type: row.1,
            status: row.2,
            period_start: row.3.map(|value| value.to_string()).unwrap_or_default(),
            period_end: row.4.map(|value| value.to_string()).unwrap_or_default(),
            created_at: row.5.to_rfc3339(),
            updated_at: row.6.to_rfc3339(),
        })
        .collect())
}

async fn recent_reconciliation_differences(
    db: &sqlx::PgPool,
) -> Result<Vec<RecentReconciliationDifference>, Status> {
    let rows = sqlx::query_as::<_, (uuid::Uuid, Option<uuid::Uuid>, Option<String>, String, String, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT brd.id, brd.tenant_id, t.name AS tenant_name,
          brd.difference_type, brd.severity, brd.created_at
        FROM billing_reconciliation_differences brd
        LEFT JOIN tenants t ON t.id = brd.tenant_id
        WHERE brd.resolved_at IS NULL
        ORDER BY brd.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| RecentReconciliationDifference {
            id: row.0.to_string(),
            tenant_id: row.1.map(|value| value.to_string()).unwrap_or_default(),
            tenant_name: row.2.unwrap_or_default(),
            difference_type: row.3,
            severity: row.4,
            created_at: row.5.to_rfc3339(),
        })
        .collect())
}
