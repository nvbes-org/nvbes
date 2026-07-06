use sqlx::Row;
use tonic::Status;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminMoneyTotal, AdminRecentCapturedPayment, AdminRecentDispute, AdminRecentDunningCase,
    AdminRecentOverdueInvoice, AdminRevenueCenterSnapshot,
};

pub async fn revenue_center(db: &sqlx::PgPool) -> Result<AdminRevenueCenterSnapshot, Status> {
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
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(AdminRevenueCenterSnapshot {
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
        recent_dunning_cases: load_recent_dunning_cases(db).await?,
        recent_disputes: load_recent_disputes(db).await?,
        recent_overdue_invoices: load_recent_overdue_invoices(db).await?,
        recent_captured_payments: load_recent_captured_payments(db).await?,
    })
}

async fn load_money_totals(
    db: &sqlx::PgPool,
    query: &'static str,
) -> Result<Vec<AdminMoneyTotal>, Status> {
    let rows = sqlx::query(query)
        .fetch_all(db)
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    Ok(rows
        .into_iter()
        .map(|row| AdminMoneyTotal {
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
            object_count: row.get("object_count"),
        })
        .collect())
}

async fn load_recent_dunning_cases(
    db: &sqlx::PgPool,
) -> Result<Vec<AdminRecentDunningCase>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT bdc.id, bdc.tenant_id, t.name AS tenant_name, bdc.status,
          bdc.policy_state, bdc.opened_at
        FROM billing_dunning_cases bdc
        JOIN tenants t ON t.id = bdc.tenant_id
        WHERE bdc.status = 'open'
        ORDER BY bdc.opened_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminRecentDunningCase {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            status: row.get("status"),
            policy_state: row.get("policy_state"),
            opened_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("opened_at")
                .to_rfc3339(),
        })
        .collect())
}

async fn load_recent_disputes(db: &sqlx::PgPool) -> Result<Vec<AdminRecentDispute>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT bd.id, bd.tenant_id, t.name AS tenant_name, bd.status,
          bd.currency, bd.amount_minor, bd.created_at
        FROM billing_disputes bd
        JOIN tenants t ON t.id = bd.tenant_id
        ORDER BY bd.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminRecentDispute {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            status: row.get("status"),
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
            created_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect())
}

async fn load_recent_overdue_invoices(
    db: &sqlx::PgPool,
) -> Result<Vec<AdminRecentOverdueInvoice>, Status> {
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
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminRecentOverdueInvoice {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            invoice_number: row
                .get::<Option<String>, _>("invoice_number")
                .unwrap_or_default(),
            status: row.get("status"),
            currency: row.get("currency"),
            total_minor: row.get("total_minor"),
            due_at: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("due_at")
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
        })
        .collect())
}

async fn load_recent_captured_payments(
    db: &sqlx::PgPool,
) -> Result<Vec<AdminRecentCapturedPayment>, Status> {
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
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminRecentCapturedPayment {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            status: row.get("status"),
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
            created_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect())
}
