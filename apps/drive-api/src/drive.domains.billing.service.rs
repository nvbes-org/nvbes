use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_core::config::AppConfig;

use super::manage;
pub use super::types::*;
use super::webhooks;

pub async fn get_billing(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
) -> Result<BillingOverviewResponse, AppError> {
    manage::get_billing(db, access).await
}

pub async fn create_checkout_session(
    db: &sqlx::PgPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: CreateCheckoutInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CheckoutSessionResponse, AppError> {
    manage::create_checkout_session(db, config, access, input, ip, user_agent).await
}

pub async fn create_portal_session(
    config: &AppConfig,
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    input: CreatePortalInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<PortalSessionResponse, AppError> {
    manage::create_portal_session(config, db, access, input, ip, user_agent).await
}

pub async fn get_usage(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
) -> Result<BillingUsageResponse, AppError> {
    manage::get_usage(db, access).await
}

pub async fn get_invoice_estimate(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
) -> Result<InvoiceEstimateResponse, AppError> {
    manage::get_invoice_estimate(db, access).await
}

pub async fn handle_webhook(
    db: &sqlx::PgPool,
    config: &AppConfig,
    signature_header: Option<&str>,
    payload: &[u8],
) -> Result<BillingWebhookResponse, AppError> {
    webhooks::handle_webhook(db, config, signature_header, payload).await
}
