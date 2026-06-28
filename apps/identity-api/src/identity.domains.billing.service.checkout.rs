use sqlx::PgPool;

use super::super::types::*;
use super::super::{db, policy, provider_mollie, provider_routing, stripe};
use crate::domains::auth::{
    risk::{self, RiskDecision, RiskEventInput},
    verification,
};
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_billing::provider::{ProviderCheckoutInput, ProviderCode};
use nvbes_billing::{plan_monthly_price_cents, validate_plan_code};
use nvbes_core::auth::Aal;
use nvbes_core::config::AppConfig;
use nvbes_core::limiter::RateLimiter;
use nvbes_region::geo::{GeoLookupPurpose, GeoLookupRecordContext, record_geo_resolution_tx};

use super::geo::{checkout_geo_risk, resolve_checkout_geo};

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
    trusted_country_header: Option<String>,
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
    let geo_resolution = resolve_checkout_geo(
        &mut tx,
        config,
        ip.as_deref(),
        trusted_country_header.as_deref(),
        record.country.as_deref(),
    )
    .await?;
    let checkout_country = geo_resolution
        .location
        .as_ref()
        .map(|location| location.country_code.as_str());

    record_geo_resolution_tx(
        &mut tx,
        GeoLookupRecordContext {
            purpose: GeoLookupPurpose::Payment,
            subject_type: Some("workspace"),
            subject_id: Some(access.workspace_id),
            request_id: None,
        },
        &geo_resolution,
    )
    .await?;

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

    let provider = provider_routing::route_provider(&provider_routing::ProviderRouteRequest {
        country: checkout_country.map(ToOwned::to_owned),
        currency: "EUR".to_string(),
        payment_method: None,
        amount_minor: plan_monthly_price_cents(&target_plan.code),
        mollie_enabled: config.billing_mollie_enabled && config.mollie_api_key.is_some(),
    });

    let checkout = match provider {
        ProviderCode::Stripe => {
            let mapping =
                db::fetch_active_price_mapping_tx(&mut tx, target_plan.plan_id, checkout_country)
                    .await?;
            let customer_id = match record
                .stripe_customer_id
                .clone()
                .or_else(|| record.billing_customer_id.clone())
            {
                Some(customer_id) => customer_id,
                None => {
                    let customer = stripe::create_stripe_customer(config, &record).await?;
                    db::upsert_billing_customer_tx(&mut tx, access.workspace_id, &customer.id)
                        .await?;
                    customer.id
                }
            };
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
            CheckoutProviderResult {
                provider: "stripe".to_string(),
                checkout_id: session.id,
                url: session.url,
                provider_customer_id: customer_id.clone(),
                provider_price_id: Some(mapping.stripe_price_id.clone()),
                price_country_code: mapping.country_code,
                pricing_region: mapping.pricing_region,
                payment_id: None,
                stripe_customer_id: customer_id,
                stripe_price_id: mapping.stripe_price_id,
            }
        }
        ProviderCode::Mollie => {
            let provider_customer_id = record
                .billing_customer_id
                .clone()
                .unwrap_or_else(|| access.workspace_id.to_string());
            let payment = provider_mollie::create_mollie_payment(
                config,
                &ProviderCheckoutInput {
                    tenant_id: record.workspace_id.to_string(),
                    provider_customer_id: provider_customer_id.clone(),
                    amount_minor: plan_monthly_price_cents(&target_plan.code),
                    currency: "EUR".to_string(),
                    success_url: success_url.clone(),
                    cancel_url: cancel_url.clone(),
                    webhook_url: Some(format!(
                        "{}/webhooks/mollie",
                        config.api_base_url.trim_end_matches('/')
                    )),
                },
            )
            .await?;
            CheckoutProviderResult {
                provider: "mollie".to_string(),
                checkout_id: payment.checkout_id.clone(),
                url: payment.url,
                provider_customer_id,
                provider_price_id: None,
                price_country_code: None,
                pricing_region: None,
                payment_id: Some(payment.checkout_id),
                stripe_customer_id: String::new(),
                stripe_price_id: String::new(),
            }
        }
    };
    let geo_risk = checkout_geo_risk(&geo_resolution, record.country.as_deref());

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
                "provider": checkout.provider,
                "provider_customer_id": checkout.provider_customer_id,
                "provider_price_id": checkout.provider_price_id,
                "geo_country_code": checkout_country,
                "geo_source": geo_resolution.source.as_str(),
                "geo_confidence": geo_resolution.confidence.as_str(),
                "geo_network_kind": geo_resolution.network_kind.as_str(),
                "geo_risk_score": geo_resolution.risk_score,
                "geo_risk_labels": geo_resolution.risk_labels.clone(),
                "geo_risk_factors": geo_risk.factors.clone(),
                "price_country_code": checkout.price_country_code,
                "pricing_region": checkout.pricing_region,
                "checkout_id": checkout.checkout_id,
                "payment_id": checkout.payment_id,
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
            risk_score: geo_risk.score,
            risk_factors: serde_json::json!({
                "workspace_id": access.workspace_id,
                "plan_code": target_plan.code,
                "geo": {
                    "country_code": checkout_country,
                    "source": geo_resolution.source.as_str(),
                    "confidence": geo_resolution.confidence.as_str(),
                    "private_network": geo_resolution.private_network,
                    "stored_country": record.country,
                    "factors": geo_risk.factors.clone(),
                },
            }),
            decision: RiskDecision::Allow,
            metadata: serde_json::json!({
                "provider": checkout.provider,
                "checkout_id": checkout.checkout_id,
            }),
        },
    )
    .await;

    Ok(CheckoutSessionResponse {
        provider: checkout.provider,
        session_id: checkout.checkout_id.clone(),
        checkout_id: checkout.checkout_id,
        url: checkout.url,
        provider_customer_id: checkout.provider_customer_id,
        provider_price_id: checkout.provider_price_id,
        payment_id: checkout.payment_id,
        stripe_customer_id: checkout.stripe_customer_id,
        stripe_price_id: checkout.stripe_price_id,
    })
}

struct CheckoutProviderResult {
    provider: String,
    checkout_id: String,
    url: String,
    provider_customer_id: String,
    provider_price_id: Option<String>,
    price_country_code: Option<String>,
    pricing_region: Option<String>,
    payment_id: Option<String>,
    stripe_customer_id: String,
    stripe_price_id: String,
}
