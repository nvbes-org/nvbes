use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct RevenueCenterSnapshot {
    captured_payments_30d: Vec<MoneyTotal>,
    open_invoices: Vec<MoneyTotal>,
    overdue_invoices: Vec<MoneyTotal>,
    refunds_30d: Vec<MoneyTotal>,
    disputes_30d: Vec<MoneyTotal>,
    active_subscription_count: i64,
    trialing_subscription_count: i64,
    open_dunning_case_count: i64,
    unresolved_reconciliation_difference_count: i64,
    recent_overdue_invoices: Vec<RecentOverdueInvoice>,
    recent_captured_payments: Vec<RecentCapturedPayment>,
}

#[derive(Debug, Serialize)]
struct MoneyTotal {
    currency: String,
    amount_minor: i64,
    object_count: i64,
}

#[derive(Debug, Serialize)]
struct RecentOverdueInvoice {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    invoice_number: Option<String>,
    status: String,
    currency: String,
    total_minor: i64,
    due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct RecentCapturedPayment {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    status: String,
    currency: String,
    amount_minor: i64,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/revenue-center", get(revenue_center_route))
}

async fn revenue_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RevenueCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_revenue_center(&state.db).await?))
}

async fn load_revenue_center(db: &PgPool) -> Result<RevenueCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM billing_subscriptions
            WHERE status = 'active'
          ) AS active_subscription_count,
          (
            SELECT COUNT(*) FROM billing_subscriptions
            WHERE status = 'trialing'
          ) AS trialing_subscription_count,
          (
            SELECT COUNT(*) FROM billing_dunning_cases
            WHERE status = 'open'
          ) AS open_dunning_case_count,
          (
            SELECT COUNT(*) FROM billing_reconciliation_differences
            WHERE resolved_at IS NULL
          ) AS unresolved_reconciliation_difference_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(RevenueCenterSnapshot {
        captured_payments_30d: load_money_totals(
            db,
            r#"
            SELECT currency, COALESCE(SUM(amount_minor), 0) AS amount_minor, COUNT(*) AS object_count
            FROM billing_payments
            WHERE status::text = 'captured' AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY currency
            ORDER BY currency
            "#,
        )
        .await?,
        open_invoices: load_money_totals(
            db,
            r#"
            SELECT currency, COALESCE(SUM(total_minor), 0) AS amount_minor, COUNT(*) AS object_count
            FROM billing_invoices
            WHERE status::text IN ('issued', 'pro_forma')
            GROUP BY currency
            ORDER BY currency
            "#,
        )
        .await?,
        overdue_invoices: load_money_totals(
            db,
            r#"
            SELECT currency, COALESCE(SUM(total_minor), 0) AS amount_minor, COUNT(*) AS object_count
            FROM billing_invoices
            WHERE due_at < NOW() AND status::text IN ('issued', 'pro_forma')
            GROUP BY currency
            ORDER BY currency
            "#,
        )
        .await?,
        refunds_30d: load_money_totals(
            db,
            r#"
            SELECT currency, COALESCE(SUM(amount_minor), 0) AS amount_minor, COUNT(*) AS object_count
            FROM billing_refunds
            WHERE created_at >= NOW() - INTERVAL '30 days'
            GROUP BY currency
            ORDER BY currency
            "#,
        )
        .await?,
        disputes_30d: load_money_totals(
            db,
            r#"
            SELECT currency, COALESCE(SUM(amount_minor), 0) AS amount_minor, COUNT(*) AS object_count
            FROM billing_disputes
            WHERE created_at >= NOW() - INTERVAL '30 days'
            GROUP BY currency
            ORDER BY currency
            "#,
        )
        .await?,
        active_subscription_count: metrics.get("active_subscription_count"),
        trialing_subscription_count: metrics.get("trialing_subscription_count"),
        open_dunning_case_count: metrics.get("open_dunning_case_count"),
        unresolved_reconciliation_difference_count: metrics
            .get("unresolved_reconciliation_difference_count"),
        recent_overdue_invoices: load_recent_overdue_invoices(db).await?,
        recent_captured_payments: load_recent_captured_payments(db).await?,
    })
}

async fn load_money_totals(db: &PgPool, query: &'static str) -> Result<Vec<MoneyTotal>, AppError> {
    let rows = sqlx::query(query).fetch_all(db).await?;
    Ok(rows
        .into_iter()
        .map(|row| MoneyTotal {
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
            object_count: row.get("object_count"),
        })
        .collect())
}

async fn load_recent_overdue_invoices(db: &PgPool) -> Result<Vec<RecentOverdueInvoice>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT bi.id, bi.tenant_id, t.name AS tenant_name, bi.invoice_number,
          bi.status::text AS status, bi.currency, bi.total_minor, bi.due_at
        FROM billing_invoices bi
        JOIN tenants t ON t.id = bi.tenant_id
        WHERE bi.due_at < NOW() AND bi.status::text IN ('issued', 'pro_forma')
        ORDER BY bi.due_at ASC NULLS LAST, bi.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentOverdueInvoice {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            invoice_number: row.get("invoice_number"),
            status: row.get("status"),
            currency: row.get("currency"),
            total_minor: row.get("total_minor"),
            due_at: row.get("due_at"),
        })
        .collect())
}

async fn load_recent_captured_payments(
    db: &PgPool,
) -> Result<Vec<RecentCapturedPayment>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT bp.id, bp.tenant_id, t.name AS tenant_name, bp.status::text AS status,
          bp.currency, bp.amount_minor, bp.created_at
        FROM billing_payments bp
        JOIN tenants t ON t.id = bp.tenant_id
        WHERE bp.status::text = 'captured'
        ORDER BY bp.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentCapturedPayment {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            status: row.get("status"),
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
            created_at: row.get("created_at"),
        })
        .collect())
}
