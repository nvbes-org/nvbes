use sqlx::PgPool;
use uuid::Uuid;

use crate::shared::current_billing_period;
use crate::types::{
    BillingOverviewResponse, BillingUsageResponse, ProductEntitlementsView, UsageLineView,
};
use crate::views::{
    billing_account_view, build_invoice_estimate_with_price, entitlements_view,
    plan_view_with_price, subscription_view,
};

pub async fn fetch_workspace_billing_overview(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<BillingOverviewResponse, sqlx::Error> {
    let mut tx = db.begin().await?;
    let record = crate::db::fetch_billing_state_tx(&mut tx, workspace_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    let price_mapping = crate::db::fetch_active_price_mapping_tx(
        &mut tx,
        record.plan_id,
        record.country.as_deref(),
    )
    .await?;
    tx.commit().await?;

    Ok(BillingOverviewResponse {
        workspace_id,
        plan: plan_view_with_price(&record, price_mapping.as_ref()),
        subscription: subscription_view(&record),
        billing_account: billing_account_view(&record),
        entitlements: entitlements_view(&record),
        invoice_estimate: build_invoice_estimate_with_price(&record, price_mapping.as_ref()),
    })
}

pub async fn fetch_workspace_billing_usage(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<BillingUsageResponse, sqlx::Error> {
    let mut tx = db.begin().await?;
    let record = crate::db::fetch_billing_state_tx(&mut tx, workspace_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    tx.commit().await?;
    let (period_start, period_end) = current_billing_period();
    let included_storage_bytes = i64::from(record.included_storage_gb) * 1024 * 1024 * 1024;

    Ok(BillingUsageResponse {
        workspace_id,
        period_start,
        period_end,
        storage: UsageLineView {
            included_quantity: included_storage_bytes,
            used_quantity: record.used_storage_bytes,
            billable_quantity: record
                .used_storage_bytes
                .saturating_sub(included_storage_bytes),
            unit: "bytes".to_string(),
        },
        seats: UsageLineView {
            included_quantity: i64::from(record.included_users),
            used_quantity: record.active_user_count,
            billable_quantity: record
                .active_user_count
                .saturating_sub(i64::from(record.included_users)),
            unit: "seat".to_string(),
        },
        bandwidth_out_bytes_month: record.bandwidth_out_bytes_month,
    })
}

pub async fn fetch_workspace_entitlements(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<ProductEntitlementsView, sqlx::Error> {
    Ok(fetch_workspace_billing_overview(db, workspace_id)
        .await?
        .entitlements)
}
