use nvbes_core::config::AppConfig;
use nvbes_region::geo::{GeoLookupPurpose, GeoLookupRecordContext, record_geo_resolution_tx};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::checkout_provider::{
    CheckoutProviderError, checkout_session_response, create_provider_checkout,
    fetch_paid_checkout_plan,
};
use crate::checkout_routing::{checkout_route_candidates, route_checkout_provider};
use crate::checkout_sessions_audit::{
    CheckoutRoutingBlockedAudit, CheckoutStartedAudit, audit_checkout_routing_blocked,
    audit_checkout_started,
};
use crate::checkout_sessions_fraud::{
    AssessCheckoutFraudSessionParams, CheckoutFraudAssessmentRecord, assess_checkout_session_fraud,
    checkout_fraud_enforcement, enforce_checkout_fraud, fraud_provider_metadata,
    record_checkout_fraud_assessment_tx,
};
use crate::db::{ProviderRoutingRuleLookupError, fetch_provider_routing_rule_tx};
use crate::models::BillingStateRecord;
use crate::types::{CheckoutSessionResponse, CreateCheckoutInput};
use crate::{
    BillingRedirectUrlError, CheckoutFraudPolicyError, ProviderRoutingError,
    plan_monthly_price_cents, resolve_billing_redirect_url, subscription_status_requires_lock,
};

#[derive(Debug, Clone)]
pub struct CreateBillingCheckoutSessionInput {
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub checkout: CreateCheckoutInput,
    pub ip: Option<String>,
    pub trusted_country_header: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Error)]
pub enum BillingCheckoutSessionError {
    #[error("workspace_not_found")]
    WorkspaceNotFound,
    #[error("billing_locked")]
    BillingLocked,
    #[error("invalid billing redirect URL in {field}")]
    InvalidRedirect {
        field: &'static str,
        env_name: &'static str,
        source: BillingRedirectUrlError,
    },
    #[error("billing fraud policy is invalid")]
    FraudPolicy(#[from] CheckoutFraudPolicyError),
    #[error("billing checkout requires manual review")]
    ManualReviewHold,
    #[error("billing checkout blocked by fraud policy")]
    FraudBlocked,
    #[error("billing checkout routing blocked: {0:?}")]
    RoutingBlocked(ProviderRoutingError),
    #[error(transparent)]
    RoutingRule(#[from] ProviderRoutingRuleLookupError),
    #[error(transparent)]
    CheckoutProvider(#[from] CheckoutProviderError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub async fn create_billing_checkout_session(
    db: &PgPool,
    config: &AppConfig,
    input: CreateBillingCheckoutSessionInput,
) -> Result<CheckoutSessionResponse, BillingCheckoutSessionError> {
    let mut tx = db.begin().await?;
    let record = crate::db::fetch_billing_state_tx(&mut tx, input.workspace_id)
        .await?
        .ok_or(BillingCheckoutSessionError::WorkspaceNotFound)?;
    enforce_billing_record_can_start_checkout(&record)?;

    let geo_resolution = crate::checkout_geo::resolve_checkout_geo(
        &mut tx,
        config,
        input.ip.as_deref(),
        input.trusted_country_header.as_deref(),
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
            subject_id: Some(input.workspace_id),
            request_id: None,
        },
        &geo_resolution,
    )
    .await?;

    let target_plan = fetch_paid_checkout_plan(&mut tx, &input.checkout).await?;
    let success_url = checkout_redirect_url(
        input.checkout.success_url.as_deref(),
        &config.billing_default_success_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "success_url",
        "NVBES_BILLING_SUCCESS_URL",
    )?;
    let cancel_url = checkout_redirect_url(
        input.checkout.cancel_url.as_deref(),
        &config.billing_default_cancel_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "cancel_url",
        "NVBES_BILLING_CANCEL_URL",
    )?;

    let checkout_amount_minor = plan_monthly_price_cents(&target_plan.code);
    let routing_rule = fetch_provider_routing_rule_tx(
        &mut tx,
        checkout_country,
        "EUR",
        "card",
        &record.customer_type,
        checkout_amount_minor,
    )
    .await?;
    let route_candidates = checkout_route_candidates(
        config,
        checkout_country,
        checkout_amount_minor,
        routing_rule.as_ref(),
    );
    let provider_decision = match route_checkout_provider(
        config,
        checkout_country,
        checkout_amount_minor,
        routing_rule.as_ref(),
    ) {
        Ok(decision) => decision,
        Err(error) => {
            audit_checkout_routing_blocked(
                &mut tx,
                CheckoutRoutingBlockedAudit {
                    workspace_id: input.workspace_id,
                    actor_principal_id: input.actor_principal_id,
                    plan_code: &target_plan.code,
                    amount_minor: checkout_amount_minor,
                    routing_error: error.as_str(),
                    checkout_country,
                    ip: input.ip.as_deref(),
                    user_agent: input.user_agent.as_deref(),
                    routing_rule_id: routing_rule.as_ref().map(|rule| rule.id),
                    routing_rule_provider: routing_rule.as_ref().map(|rule| rule.provider.as_str()),
                    route_candidates,
                },
                config,
            )
            .await?;
            tx.commit().await?;
            return Err(BillingCheckoutSessionError::RoutingBlocked(error));
        }
    };

    let fraud_ip_address = geo_resolution.ip.map(|ip| ip.to_string());
    let fraud_velocity = crate::db::billing_fraud_velocity_tx(
        &mut tx,
        input.workspace_id,
        fraud_ip_address.as_deref(),
        Some(provider_decision.provider.as_str()),
    )
    .await?;
    let fraud_assessment = assess_checkout_session_fraud(AssessCheckoutFraudSessionParams {
        config,
        record: &record,
        target_plan: &target_plan,
        provider: provider_decision.provider,
        checkout_country,
        checkout_amount_minor,
        geo_resolution: &geo_resolution,
        velocity: fraud_velocity,
    })?;
    let fraud_enforcement = checkout_fraud_enforcement(config, &fraud_assessment);
    if let Err(error) = enforce_checkout_fraud(fraud_enforcement) {
        record_checkout_fraud_assessment_tx(
            &mut tx,
            CheckoutFraudAssessmentRecord {
                workspace_id: input.workspace_id,
                actor_principal_id: input.actor_principal_id,
                record: &record,
                target_plan: &target_plan,
                checkout_amount_minor,
                ip_address: fraud_ip_address.as_deref(),
                checkout_country,
                provider: Some(provider_decision.provider.as_str()),
                checkout_id: None,
                payment_id: None,
                geo_resolution: &geo_resolution,
                assessment: &fraud_assessment,
                action: fraud_enforcement,
                velocity: fraud_velocity,
            },
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }

    let fraud_metadata = fraud_provider_metadata(&fraud_assessment, fraud_enforcement);
    let checkout = create_provider_checkout(
        &mut tx,
        config,
        crate::checkout_provider::CreateProviderCheckoutInput {
            workspace_id: input.workspace_id,
            record: &record,
            target_plan: &target_plan,
            checkout_country,
            checkout_amount_minor,
            success_url: &success_url,
            cancel_url: &cancel_url,
            provider: provider_decision.provider,
            fraud_metadata: &fraud_metadata,
        },
    )
    .await?;
    record_checkout_fraud_assessment_tx(
        &mut tx,
        CheckoutFraudAssessmentRecord {
            workspace_id: input.workspace_id,
            actor_principal_id: input.actor_principal_id,
            record: &record,
            target_plan: &target_plan,
            checkout_amount_minor,
            ip_address: fraud_ip_address.as_deref(),
            checkout_country,
            provider: Some(checkout.provider.as_str()),
            checkout_id: Some(&checkout.checkout_id),
            payment_id: checkout.payment_id.as_deref(),
            geo_resolution: &geo_resolution,
            assessment: &fraud_assessment,
            action: fraud_enforcement,
            velocity: fraud_velocity,
        },
    )
    .await?;
    audit_checkout_started(
        &mut tx,
        CheckoutStartedAudit {
            workspace_id: input.workspace_id,
            actor_principal_id: input.actor_principal_id,
            plan_code: &target_plan.code,
            checkout_country,
            ip: input.ip.as_deref(),
            user_agent: input.user_agent.as_deref(),
            provider_decision: &provider_decision,
            route_candidates,
            routing_rule_id: routing_rule.as_ref().map(|rule| rule.id),
            routing_rule_provider: routing_rule.as_ref().map(|rule| rule.provider.as_str()),
            checkout: &checkout,
            fraud_assessment: &fraud_assessment,
            fraud_enforcement,
            geo_source: geo_resolution.source.as_str(),
            geo_confidence: geo_resolution.confidence.as_str(),
            network_kind: geo_resolution.network_kind.as_str(),
            network_risk_score: geo_resolution.risk_score,
            network_risk_labels: &geo_resolution.risk_labels,
        },
    )
    .await?;

    tx.commit().await?;
    Ok(checkout_session_response(checkout))
}

fn enforce_billing_record_can_start_checkout(
    record: &BillingStateRecord,
) -> Result<(), BillingCheckoutSessionError> {
    if subscription_status_requires_lock(&record.subscription_status) {
        return Err(BillingCheckoutSessionError::BillingLocked);
    }
    Ok(())
}

fn checkout_redirect_url(
    value: Option<&str>,
    default_url: &str,
    primary_origin: &str,
    staging_origin: Option<&str>,
    field: &'static str,
    env_name: &'static str,
) -> Result<String, BillingCheckoutSessionError> {
    resolve_billing_redirect_url(value, default_url, primary_origin, staging_origin).map_err(
        |source| BillingCheckoutSessionError::InvalidRedirect {
            field,
            env_name,
            source,
        },
    )
}
