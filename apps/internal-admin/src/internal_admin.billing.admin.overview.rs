use axum::{Json, Router, extract::Path, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct BillingOverview {
    open_invoice_count: i64,
    overdue_invoice_count: i64,
    open_invoice_total_minor: i64,
    failed_provider_event_count: i64,
    pending_refund_count: i64,
    active_subscription_count: i64,
    captured_payment_total_minor_30d: i64,
    last_billing_audit_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/billing/admin/overview",
        get(overview_route),
    )
}

async fn overview_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingOverview>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        load_billing_overview(&state.db, access.tenant_id).await?,
    ))
}

async fn load_billing_overview(db: &PgPool, tenant_id: Uuid) -> Result<BillingOverview, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM billing_invoices
            WHERE tenant_id = $1 AND status::text IN ('issued', 'pro_forma')
          ) AS open_invoice_count,
          (
            SELECT COUNT(*) FROM billing_invoices
            WHERE tenant_id = $1 AND due_at < NOW() AND status::text IN ('issued', 'pro_forma')
          ) AS overdue_invoice_count,
          (
            SELECT COALESCE(SUM(total_minor), 0) FROM billing_invoices
            WHERE tenant_id = $1 AND status::text IN ('issued', 'pro_forma')
          ) AS open_invoice_total_minor,
          (
            SELECT COUNT(*) FROM billing_provider_events
            WHERE tenant_id = $1 AND status::text IN ('failed', 'rejected')
          ) AS failed_provider_event_count,
          (
            SELECT COUNT(*) FROM billing_refunds
            WHERE tenant_id = $1 AND status = 'pending'
          ) AS pending_refund_count,
          (
            SELECT COUNT(*) FROM billing_subscriptions
            WHERE tenant_id = $1 AND status IN ('active', 'trialing')
          ) AS active_subscription_count,
          (
            SELECT COALESCE(SUM(amount_minor), 0) FROM billing_payments
            WHERE tenant_id = $1 AND status::text = 'captured'
              AND created_at >= NOW() - INTERVAL '30 days'
          ) AS captured_payment_total_minor_30d,
          (
            SELECT MAX(created_at) FROM audit_events
            WHERE tenant_id = $1 AND action LIKE 'billing.%'
          ) AS last_billing_audit_at
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?;

    Ok(BillingOverview {
        open_invoice_count: row.get("open_invoice_count"),
        overdue_invoice_count: row.get("overdue_invoice_count"),
        open_invoice_total_minor: row.get("open_invoice_total_minor"),
        failed_provider_event_count: row.get("failed_provider_event_count"),
        pending_refund_count: row.get("pending_refund_count"),
        active_subscription_count: row.get("active_subscription_count"),
        captured_payment_total_minor_30d: row.get("captured_payment_total_minor_30d"),
        last_billing_audit_at: row.get("last_billing_audit_at"),
    })
}
