use sqlx::PgPool;

use super::super::types::*;
use super::super::{db, policy, stripe};
use crate::domains::auth::{
    risk::{self, RiskDecision, RiskEventInput},
    verification,
};
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_billing::validate_plan_code;
use nvbes_core::auth::Aal;
use nvbes_core::config::AppConfig;
use nvbes_core::limiter::RateLimiter;

#[expect(
    clippy::too_many_arguments,
    reason = "Checkout creation keeps limiter, auth, config, and request metadata explicit."
)]
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
        AuditEventInput {
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
