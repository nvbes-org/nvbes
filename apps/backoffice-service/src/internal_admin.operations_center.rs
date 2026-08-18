use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::billing_grpc::{
    BackofficeBillingOperationsSnapshot, BackofficeRecentExportRun,
    BackofficeRecentProviderFailure, BackofficeRecentReconciliationDifference,
    get_admin_operations_center,
};
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
    open_incident_count: i64,
    scheduled_maintenance_window_count: i64,
    failed_job_run_count: i64,
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
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_operations_center(&state, actor_id).await?))
}

async fn load_operations_center(
    state: &AppState,
    actor_id: Uuid,
) -> Result<OperationsCenterSnapshot, AppError> {
    let billing = get_admin_operations_center(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id: actor_id,
        },
    )
    .await?;
    let email = state.email_operations.snapshot(actor_id).await?;
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM internal_admin_incidents
            WHERE status IN ('open', 'mitigating')
          ) AS open_incident_count,
          (
            SELECT COUNT(*) FROM internal_admin_maintenance_windows
            WHERE status = 'scheduled' AND scheduled_end_at >= NOW()
          ) AS scheduled_maintenance_window_count,
          (
            SELECT COUNT(*) FROM internal_admin_job_runs
            WHERE status = 'failed'
          ) AS failed_job_run_count,
          (
            SELECT COUNT(*) FROM audit_events
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(snapshot_from_metrics(metrics, billing, email))
}

fn snapshot_from_metrics(
    metrics: sqlx::postgres::PgRow,
    billing: BackofficeBillingOperationsSnapshot,
    email: nvbes_email::proto::nvbes::email::v1::EmailOperationsSnapshot,
) -> OperationsCenterSnapshot {
    OperationsCenterSnapshot {
        provider_event_failure_count: billing.provider_event_failure_count,
        provider_event_backlog_count: billing.provider_event_backlog_count,
        export_pending_count: billing.export_pending_count,
        export_failed_count: billing.export_failed_count,
        reconciliation_pending_count: billing.reconciliation_pending_count,
        reconciliation_failed_count: billing.reconciliation_failed_count,
        unresolved_reconciliation_difference_count: billing
            .unresolved_reconciliation_difference_count,
        open_incident_count: metrics.get("open_incident_count"),
        scheduled_maintenance_window_count: metrics.get("scheduled_maintenance_window_count"),
        failed_job_run_count: metrics.get("failed_job_run_count"),
        queued_email_count: email.queued_message_count,
        dropped_email_count_24h: email.failed_message_count_24h,
        audit_events_24h: metrics.get("audit_events_24h"),
        recent_provider_failures: billing
            .recent_provider_failures
            .into_iter()
            .map(RecentProviderFailure::from)
            .collect(),
        recent_export_runs: billing
            .recent_export_runs
            .into_iter()
            .map(RecentExportRun::from)
            .collect(),
        recent_reconciliation_differences: billing
            .recent_reconciliation_differences
            .into_iter()
            .map(RecentReconciliationDifference::from)
            .collect(),
    }
}

impl From<BackofficeRecentProviderFailure> for RecentProviderFailure {
    fn from(value: BackofficeRecentProviderFailure) -> Self {
        Self {
            id: value.id,
            tenant_id: value.tenant_id,
            tenant_name: value.tenant_name,
            provider: value.provider,
            provider_event_id: value.provider_event_id,
            event_type: value.event_type,
            status: value.status,
            received_at: value.received_at,
        }
    }
}

impl From<BackofficeRecentExportRun> for RecentExportRun {
    fn from(value: BackofficeRecentExportRun) -> Self {
        Self {
            id: value.id,
            export_type: value.export_type,
            status: value.status,
            period_start: value.period_start,
            period_end: value.period_end,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<BackofficeRecentReconciliationDifference> for RecentReconciliationDifference {
    fn from(value: BackofficeRecentReconciliationDifference) -> Self {
        Self {
            id: value.id,
            tenant_id: value.tenant_id,
            tenant_name: value.tenant_name,
            difference_type: value.difference_type,
            severity: value.severity,
            created_at: value.created_at,
        }
    }
}
