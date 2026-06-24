use nvbes_billing::{
    billing_account_view, build_invoice_estimate_with_price, current_billing_period,
    entitlements_view, plan_view_with_price, subscription_view,
};
use sqlx::PgPool;

use super::db::fetch_billing_state_tx;
use super::entitlements::persist_invoice_estimate;
use super::types::*;
use super::usage_events::{DriveUsageEventInput, build_drive_usage_event};
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

pub async fn get_billing(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<BillingOverviewResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let record = fetch_billing_state_tx(&mut tx, access.workspace_id).await?;
    let price_mapping = nvbes_billing::db::fetch_active_price_mapping_tx(
        &mut tx,
        record.plan_id,
        record.country.as_deref(),
    )
    .await?;
    let estimate = build_invoice_estimate_with_price(&record, price_mapping.as_ref());
    tx.commit().await?;

    Ok(BillingOverviewResponse {
        workspace_id: access.workspace_id,
        plan: plan_view_with_price(&record, price_mapping.as_ref()),
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
    if let Some(tenant_id) = access.tenant_id {
        let _storage_snapshot_event = build_drive_usage_event(DriveUsageEventInput {
            tenant_id,
            workspace_id: access.workspace_id,
            meter_code: "storage_gb_month".to_string(),
            quantity: record.used_storage_bytes,
            unit: "bytes".to_string(),
            occurred_at: chrono::Utc::now(),
            source_operation_id: format!("usage-view:{}:{period_start}", access.workspace_id),
        });
    }

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
    let price_mapping = nvbes_billing::db::fetch_active_price_mapping_tx(
        &mut tx,
        record.plan_id,
        record.country.as_deref(),
    )
    .await?;
    tx.commit().await?;

    let estimate = build_invoice_estimate_with_price(&record, price_mapping.as_ref());
    persist_invoice_estimate(db, &estimate).await?;

    Ok(InvoiceEstimateResponse { estimate })
}
