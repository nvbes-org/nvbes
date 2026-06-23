use axum::{Json, Router, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;
use axum::extract::State;

#[derive(Debug, Serialize)]
struct CommandCenterSnapshot {
    tenant_count: i64,
    workspace_count: i64,
    user_count: i64,
    audit_events_24h: i64,
    billing_provider_failures: i64,
    overdue_invoice_count: i64,
    failed_payment_count: i64,
    latest_audit_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/command-center", get(command_center_route))
}

async fn command_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CommandCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_command_center_snapshot(&state.db).await?))
}

async fn load_command_center_snapshot(db: &PgPool) -> Result<CommandCenterSnapshot, AppError> {
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
            SELECT COUNT(*) FROM billing_provider_events
            WHERE status::text IN ('failed', 'rejected')
          ) AS billing_provider_failures,
          (
            SELECT COUNT(*) FROM billing_invoices
            WHERE due_at < NOW() AND status::text IN ('issued', 'pro_forma')
          ) AS overdue_invoice_count,
          (
            SELECT COUNT(*) FROM billing_payments
            WHERE status::text IN ('failed', 'disputed')
          ) AS failed_payment_count,
          (SELECT MAX(created_at) FROM audit_events) AS latest_audit_at
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(CommandCenterSnapshot {
        tenant_count: row.get("tenant_count"),
        workspace_count: row.get("workspace_count"),
        user_count: row.get("user_count"),
        audit_events_24h: row.get("audit_events_24h"),
        billing_provider_failures: row.get("billing_provider_failures"),
        overdue_invoice_count: row.get("overdue_invoice_count"),
        failed_payment_count: row.get("failed_payment_count"),
        latest_audit_at: row.get("latest_audit_at"),
    })
}
