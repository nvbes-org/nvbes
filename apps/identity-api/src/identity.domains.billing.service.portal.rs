use sqlx::PgPool;

use super::super::types::*;
use super::super::{db, policy, stripe};
use crate::domains::auth::{
    risk::{self, RiskDecision, RiskEventInput},
    verification,
};
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_core::auth::Aal;
use nvbes_core::config::AppConfig;
use nvbes_core::limiter::RateLimiter;

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
        AuditEventInput {
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
