use axum::{Json, Router, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use axum::extract::State;

#[derive(Debug, Serialize)]
struct CommandCenterSnapshot {
    tenant_count: i64,
    workspace_count: i64,
    user_count: i64,
    audit_events_24h: i64,
    pending_approval_count: i64,
    critical_pending_approval_count: i64,
    overdue_approval_count: i64,
    open_incident_count: i64,
    audit_hash_anomaly_count: i64,
    sla_breach_count: i64,
    billing_provider_failures: i64,
    overdue_invoice_count: i64,
    failed_payment_count: i64,
    latest_audit_at: Option<DateTime<Utc>>,
    today_work: Vec<CommandCenterWorkItem>,
}

#[derive(Debug, Serialize)]
struct CommandCenterWorkItem {
    id: &'static str,
    label: &'static str,
    count: i64,
    severity: &'static str,
    href: &'static str,
    owner: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/command-center", get(command_center_route))
}

async fn command_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CommandCenterSnapshot>, AppError> {
    let actor_principal_id = actor_principal_id(&headers)?;
    Ok(Json(
        load_command_center_snapshot(&state, actor_principal_id).await?,
    ))
}

async fn load_command_center_snapshot(
    state: &AppState,
    actor_principal_id: Uuid,
) -> Result<CommandCenterSnapshot, AppError> {
    let db = &state.db;
    let billing_metrics = crate::billing_grpc::get_admin_command_center_billing_metrics(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await?;
    let row = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM tenants) AS tenant_count,
          (SELECT COUNT(*) FROM workspaces) AS workspace_count,
          (SELECT COUNT(*) FROM users) AS user_count,
          (
            SELECT COUNT(*) FROM audit_events
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS audit_events_24h,
          (
            (SELECT COUNT(*) FROM developer_marketplace_apps WHERE status::text = 'pending') +
            (SELECT COUNT(*) FROM enterprise_password_recovery_requests WHERE status = 'pending')
          ) AS pending_approval_count,
          (
            SELECT COUNT(*) FROM enterprise_password_recovery_requests WHERE status = 'pending'
          ) AS critical_pending_approval_count,
          (
            SELECT COUNT(*) FROM enterprise_password_recovery_requests
            WHERE status = 'pending' AND available_at < NOW()
          ) AS overdue_approval_count,
          (
            SELECT COUNT(*) FROM internal_admin_incidents
            WHERE status::text NOT IN ('resolved', 'closed')
          ) AS open_incident_count,
          (
            SELECT COUNT(*) FROM audit_events
            WHERE event_hash IS NULL OR event_hash = '' OR event_hash = 'backfill'
          ) AS audit_hash_anomaly_count,
          (SELECT MAX(created_at) FROM audit_events) AS latest_audit_at
        "#,
    )
    .fetch_one(db)
    .await?;

    let pending_approval_count: i64 =
        row.get::<i64, _>("pending_approval_count") + billing_metrics.pending_kyc_approval_count;
    let critical_pending_approval_count = row.get("critical_pending_approval_count");
    let overdue_approval_count = row.get("overdue_approval_count");
    let open_incident_count = row.get("open_incident_count");
    let audit_hash_anomaly_count = row.get("audit_hash_anomaly_count");
    let billing_provider_failures = billing_metrics.provider_failure_count;
    let overdue_invoice_count = billing_metrics.overdue_invoice_count;
    let failed_payment_count = billing_metrics.failed_payment_count;
    let sla_breach_count = open_incident_count
        + billing_provider_failures
        + overdue_invoice_count
        + failed_payment_count;

    Ok(CommandCenterSnapshot {
        tenant_count: row.get("tenant_count"),
        workspace_count: row.get("workspace_count"),
        user_count: row.get("user_count"),
        audit_events_24h: row.get("audit_events_24h"),
        pending_approval_count,
        critical_pending_approval_count,
        overdue_approval_count,
        open_incident_count,
        audit_hash_anomaly_count,
        sla_breach_count,
        billing_provider_failures,
        overdue_invoice_count,
        failed_payment_count,
        latest_audit_at: row.get("latest_audit_at"),
        today_work: command_center_work_items(
            pending_approval_count,
            critical_pending_approval_count,
            overdue_approval_count,
            open_incident_count,
            audit_hash_anomaly_count,
            sla_breach_count,
        ),
    })
}

fn command_center_work_items(
    pending_approval_count: i64,
    critical_pending_approval_count: i64,
    overdue_approval_count: i64,
    open_incident_count: i64,
    audit_hash_anomaly_count: i64,
    sla_breach_count: i64,
) -> Vec<CommandCenterWorkItem> {
    let mut items = Vec::new();
    push_work_item(
        &mut items,
        "pending-approvals",
        "Pending approvals",
        pending_approval_count,
        severity_for_pending(critical_pending_approval_count, overdue_approval_count),
        "#pending-approvals",
        "Ops lead",
    );
    push_work_item(
        &mut items,
        "open-incidents",
        "Open incidents",
        open_incident_count,
        "critical",
        "#operations-center",
        "Operations",
    );
    push_work_item(
        &mut items,
        "audit-anomalies",
        "Audit anomalies",
        audit_hash_anomaly_count,
        "critical",
        "#audit-evidence-center",
        "Security",
    );
    push_work_item(
        &mut items,
        "sla-breaches",
        "SLA pressure",
        sla_breach_count,
        "high",
        "#revenue-center",
        "Revenue ops",
    );
    items
}

fn push_work_item(
    items: &mut Vec<CommandCenterWorkItem>,
    id: &'static str,
    label: &'static str,
    count: i64,
    severity: &'static str,
    href: &'static str,
    owner: &'static str,
) {
    if count <= 0 {
        return;
    }
    items.push(CommandCenterWorkItem {
        id,
        label,
        count,
        severity,
        href,
        owner,
    });
}

fn severity_for_pending(critical_count: i64, overdue_count: i64) -> &'static str {
    if overdue_count > 0 || critical_count > 0 {
        return "critical";
    }
    "high"
}
