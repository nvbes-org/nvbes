use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct OperationsCenterSnapshot {
    provider_event_failure_count: i64,
    provider_event_backlog_count: i64,
    export_pending_count: i64,
    export_failed_count: i64,
    reconciliation_pending_count: i64,
    reconciliation_failed_count: i64,
    unresolved_reconciliation_difference_count: i64,
    queued_email_count: i64,
    dropped_email_count_24h: i64,
    audit_events_24h: i64,
    recent_provider_failures: Vec<RecentProviderFailure>,
    recent_export_runs: Vec<RecentExportRun>,
    recent_reconciliation_differences: Vec<RecentReconciliationDifference>,
}

#[derive(Debug, Serialize)]
struct RecentProviderFailure {
    id: Uuid,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
    provider: String,
    provider_event_id: String,
    event_type: String,
    status: String,
    received_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentExportRun {
    id: Uuid,
    export_type: String,
    status: String,
    period_start: Option<chrono::NaiveDate>,
    period_end: Option<chrono::NaiveDate>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentReconciliationDifference {
    id: Uuid,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
    difference_type: String,
    severity: String,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/operations-center", get(operations_center_route))
}

async fn operations_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OperationsCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_operations_center(&state.db).await?))
}

async fn load_operations_center(db: &PgPool) -> Result<OperationsCenterSnapshot, AppError> {
    let metrics = sqlx::query(
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
          ) AS unresolved_reconciliation_difference_count,
          (
            SELECT COUNT(*) FROM email_messages
            WHERE status::text = 'queued'
          ) AS queued_email_count,
          (
            SELECT COUNT(*) FROM email_messages
            WHERE status::text IN ('bounced', 'complained', 'dropped')
              AND updated_at >= NOW() - INTERVAL '24 hours'
          ) AS dropped_email_count_24h,
          (
            SELECT COUNT(*) FROM audit_events
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(OperationsCenterSnapshot {
        provider_event_failure_count: metrics.get("provider_event_failure_count"),
        provider_event_backlog_count: metrics.get("provider_event_backlog_count"),
        export_pending_count: metrics.get("export_pending_count"),
        export_failed_count: metrics.get("export_failed_count"),
        reconciliation_pending_count: metrics.get("reconciliation_pending_count"),
        reconciliation_failed_count: metrics.get("reconciliation_failed_count"),
        unresolved_reconciliation_difference_count: metrics
            .get("unresolved_reconciliation_difference_count"),
        queued_email_count: metrics.get("queued_email_count"),
        dropped_email_count_24h: metrics.get("dropped_email_count_24h"),
        audit_events_24h: metrics.get("audit_events_24h"),
        recent_provider_failures: load_recent_provider_failures(db).await?,
        recent_export_runs: load_recent_export_runs(db).await?,
        recent_reconciliation_differences: load_recent_reconciliation_differences(db).await?,
    })
}

async fn load_recent_provider_failures(
    db: &PgPool,
) -> Result<Vec<RecentProviderFailure>, AppError> {
    let rows = sqlx::query(
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
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentProviderFailure {
            id: row.get(0),
            tenant_id: row.get(1),
            tenant_name: row.get(2),
            provider: row.get(3),
            provider_event_id: row.get(4),
            event_type: row.get(5),
            status: row.get(6),
            received_at: row.get(7),
        })
        .collect())
}

async fn load_recent_export_runs(db: &PgPool) -> Result<Vec<RecentExportRun>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, export_type, status, period_start, period_end, created_at, updated_at
        FROM billing_export_runs
        ORDER BY created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentExportRun {
            id: row.get("id"),
            export_type: row.get("export_type"),
            status: row.get("status"),
            period_start: row.get("period_start"),
            period_end: row.get("period_end"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_recent_reconciliation_differences(
    db: &PgPool,
) -> Result<Vec<RecentReconciliationDifference>, AppError> {
    let rows = sqlx::query(
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
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentReconciliationDifference {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            difference_type: row.get("difference_type"),
            severity: row.get("severity"),
            created_at: row.get("created_at"),
        })
        .collect())
}
