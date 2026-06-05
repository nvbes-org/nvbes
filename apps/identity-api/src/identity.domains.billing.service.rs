use sqlx::PgPool;
use uuid::Uuid;

pub use super::types::*;
use super::{db, policy, stripe, webhooks};
use crate::domains::auth::{
    risk::{self, RiskDecision, RiskEventInput},
    verification,
};
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_billing::{
    billing_account_view, build_invoice_estimate, current_billing_period, entitlements_view,
    plan_view, subscription_view, validate_plan_code,
};
use nvbes_core::auth::Aal;
use nvbes_core::config::AppConfig;
use nvbes_core::limiter::RateLimiter;
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

pub async fn create_checkout_session(
    limiter: &RateLimiter,
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: CreateCheckoutInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CheckoutSessionResponse, AppError> {
    verification::require_recent_step_up(redis, &access.auth, Some(Aal::Aal2)).await?;

    policy::enforce_billing_rate_limits(
        limiter,
        access.workspace_id,
        access.auth.user_id,
        "checkout",
    )
    .await?;

    let mut tx = db.begin().await?;
    let record = db::fetch_billing_state_tx(&mut tx, access.workspace_id).await?;

    policy::enforce_billing_risk_policy(db, access.workspace_id, access.auth.user_id, &record)
        .await?;

    let target_plan_code = validate_plan_code(&input.plan_code)
        .ok_or_else(|| AppError::bad_request("invalid_plan", "Unsupported billing plan."))?;
    let target_plan = db::fetch_plan_by_code_tx(&mut tx, &target_plan_code).await?;

    if target_plan.code == "trial" {
        return Err(AppError::bad_request(
            "invalid_plan",
            "Checkout cannot be created for the trial plan.",
        ));
    }

    let mapping = db::fetch_active_price_mapping_tx(&mut tx, target_plan.plan_id).await?;
    let customer_id = match record
        .stripe_customer_id
        .clone()
        .or_else(|| record.billing_customer_id.clone())
    {
        Some(customer_id) => customer_id,
        None => {
            let customer = stripe::create_stripe_customer(config, &record).await?;
            db::upsert_billing_customer_tx(&mut tx, access.workspace_id, &customer.id).await?;
            customer.id
        }
    };

    let success_url = policy::resolve_billing_redirect_url(
        input.success_url.as_deref(),
        &config.billing_default_success_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "success_url",
        "NVBES_BILLING_SUCCESS_URL",
    )?;
    let cancel_url = policy::resolve_billing_redirect_url(
        input.cancel_url.as_deref(),
        &config.billing_default_cancel_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "cancel_url",
        "NVBES_BILLING_CANCEL_URL",
    )?;

    let session = stripe::create_stripe_checkout_session(
        config,
        &customer_id,
        record.owner_principal_id,
        access.workspace_id,
        &target_plan.code,
        &mapping.stripe_price_id,
        &success_url,
        &cancel_url,
    )
    .await?;

    db::insert_audit_event(
        &mut tx,
        super::types::AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: Some(access.auth.user_id),
            action: "billing.checkout_started",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "plan_code": target_plan.code,
                "stripe_customer_id": customer_id,
                "stripe_price_id": mapping.stripe_price_id,
                "stripe_product_id": mapping.stripe_product_id,
                "checkout_session_id": session.id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: access.auth.user_id,
            session_id: Some(access.auth.session_id),
            device_id: None,
            event_type: "billing_checkout_started".to_string(),
            ip_address: ip.clone(),
            user_agent: user_agent.clone(),
            risk_score: 5.0,
            risk_factors: serde_json::json!({
                "workspace_id": access.workspace_id,
                "plan_code": target_plan.code,
            }),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({
                "checkout_session_id": session.id,
            }),
        },
    )
    .await;

    Ok(CheckoutSessionResponse {
        provider: "stripe".to_string(),
        session_id: session.id,
        url: session.url,
        stripe_customer_id: customer_id,
        stripe_price_id: mapping.stripe_price_id,
    })
}

pub async fn create_portal_session(
    limiter: &RateLimiter,
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: CreatePortalInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<PortalSessionResponse, AppError> {
    verification::require_recent_step_up(redis, &access.auth, Some(Aal::Aal2)).await?;

    policy::enforce_billing_rate_limits(
        limiter,
        access.workspace_id,
        access.auth.user_id,
        "portal",
    )
    .await?;

    let mut tx = db.begin().await?;
    let record = db::fetch_billing_state_tx(&mut tx, access.workspace_id).await?;

    policy::enforce_billing_risk_policy(db, access.workspace_id, access.auth.user_id, &record)
        .await?;

    let customer_id = record
        .stripe_customer_id
        .clone()
        .or_else(|| record.billing_customer_id.clone())
        .ok_or_else(|| {
            AppError::conflict(
                "missing_billing_customer",
                "Create a checkout session before opening the billing portal.",
            )
        })?;
    let return_url = policy::resolve_billing_redirect_url(
        input.return_url.as_deref(),
        &config.billing_default_portal_return_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "return_url",
        "NVBES_BILLING_PORTAL_RETURN_URL",
    )?;

    let session = stripe::create_stripe_portal_session(config, &customer_id, &return_url).await?;

    db::insert_audit_event(
        &mut tx,
        super::types::AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: Some(access.auth.user_id),
            action: "billing.portal_opened",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "stripe_customer_id": customer_id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let _ = risk::record_event(
        db,
        RiskEventInput {
            principal_id: access.auth.user_id,
            session_id: Some(access.auth.session_id),
            device_id: None,
            event_type: "billing_portal_opened".to_string(),
            ip_address: ip.clone(),
            user_agent: user_agent.clone(),
            risk_score: 5.0,
            risk_factors: serde_json::json!({
                "workspace_id": access.workspace_id,
            }),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({}),
        },
    )
    .await;

    Ok(PortalSessionResponse {
        provider: "stripe".to_string(),
        url: session.url,
        stripe_customer_id: customer_id,
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
