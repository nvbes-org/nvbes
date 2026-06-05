use nvbes_billing::{
    billing_account_view, build_invoice_estimate, current_billing_period, entitlements_view,
    plan_view, subscription_view,
};
use sqlx::PgPool;

use super::db::fetch_billing_state_tx;
use super::entitlements::persist_invoice_estimate;
use super::types::*;
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

pub async fn get_billing(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<BillingOverviewResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let record = fetch_billing_state_tx(&mut tx, access.workspace_id).await?;
    let estimate = build_invoice_estimate(&record);
    tx.commit().await?;

    Ok(BillingOverviewResponse {
        workspace_id: access.workspace_id,
        plan: plan_view(&record),
        subscription: subscription_view(&record),
        billing_account: billing_account_view(&record),
        entitlements: entitlements_view(&record),
        invoice_estimate: estimate,
    })
}

pub async fn get_usage(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<BillingUsageResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let record = fetch_billing_state_tx(&mut tx, access.workspace_id).await?;
    tx.commit().await?;

    let (period_start, period_end) = current_billing_period();
    let included_storage_bytes = i64::from(record.included_storage_gb) * 1024 * 1024 * 1024;

    Ok(BillingUsageResponse {
        workspace_id: access.workspace_id,
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

pub async fn get_invoice_estimate(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<InvoiceEstimateResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let record = fetch_billing_state_tx(&mut tx, access.workspace_id).await?;
    tx.commit().await?;

    let estimate = build_invoice_estimate(&record);
    persist_invoice_estimate(db, &estimate).await?;

    Ok(InvoiceEstimateResponse { estimate })
}
