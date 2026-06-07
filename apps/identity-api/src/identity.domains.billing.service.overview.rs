use sqlx::PgPool;
use uuid::Uuid;

use super::super::types::*;
use super::super::{db, webhooks};
use crate::http::error::AppError;
use nvbes_billing::{
    billing_account_view, build_invoice_estimate, current_billing_period, entitlements_view,
    plan_view, subscription_view,
};
use nvbes_core::config::AppConfig;
use nvbes_observability::metrics::HttpMetrics;

pub async fn get_billing(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<BillingOverviewResponse, AppError> {
    let record = db::fetch_billing_state_pool(db, workspace_id).await?;
    let estimate = build_invoice_estimate(&record);

    Ok(BillingOverviewResponse {
        workspace_id,
        plan: plan_view(&record),
        subscription: subscription_view(&record),
        billing_account: billing_account_view(&record),
        entitlements: entitlements_view(&record),
        invoice_estimate: estimate,
    })
}

pub async fn get_usage(db: &PgPool, workspace_id: Uuid) -> Result<BillingUsageResponse, AppError> {
    let record = db::fetch_billing_state_pool(db, workspace_id).await?;
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

pub async fn handle_webhook(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    observability: &HttpMetrics,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<BillingWebhookResponse, AppError> {
    webhooks::handle_webhook(db, redis, config, observability, signature_header, payload).await
}
