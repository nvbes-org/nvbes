use nvbes_core::config::AppConfig;
use uuid::Uuid;

use crate::checkout_routing::CheckoutProviderResult;
use crate::db::{BillingAuditEventInput, insert_billing_audit_event_tx};
use crate::{CheckoutFraudAssessment, CheckoutFraudEnforcementAction, ProviderRouteDecision};

pub(crate) struct CheckoutRoutingBlockedAudit<'a> {
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub plan_code: &'a str,
    pub amount_minor: i64,
    pub routing_error: &'a str,
    pub checkout_country: Option<&'a str>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub routing_rule_id: Option<Uuid>,
    pub routing_rule_provider: Option<&'a str>,
    pub route_candidates: serde_json::Value,
}

pub(crate) async fn audit_checkout_routing_blocked(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: CheckoutRoutingBlockedAudit<'_>,
    config: &AppConfig,
) -> Result<(), sqlx::Error> {
    insert_billing_audit_event_tx(
        tx,
        BillingAuditEventInput {
            workspace_id: input.workspace_id,
            actor_principal_id: Some(input.actor_principal_id),
            action: "billing.checkout_routing_blocked",
            target_type: "workspace",
            target_id: Some(input.workspace_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "plan_code": input.plan_code,
                "amount_minor": input.amount_minor,
                "routing_error": input.routing_error,
                "geo_country_code": input.checkout_country,
                "mollie_enabled": config.billing_mollie_enabled && config.mollie_api_key.is_some(),
                "mollie_routing_status": config.billing_mollie_routing_status,
                "external_provider_fallback_enabled": config.billing_external_provider_fallback_enabled,
                "external_provider_routing_status": config.billing_external_provider_routing_status,
                "routing_rule_id": input.routing_rule_id,
                "routing_rule_provider": input.routing_rule_provider,
                "route_candidates": input.route_candidates,
            }),
        },
    )
    .await
}

pub(crate) struct CheckoutStartedAudit<'a> {
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub plan_code: &'a str,
    pub checkout_country: Option<&'a str>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub provider_decision: &'a ProviderRouteDecision,
    pub route_candidates: serde_json::Value,
    pub routing_rule_id: Option<Uuid>,
    pub routing_rule_provider: Option<&'a str>,
    pub checkout: &'a CheckoutProviderResult,
    pub fraud_assessment: &'a CheckoutFraudAssessment,
    pub fraud_enforcement: CheckoutFraudEnforcementAction,
    pub geo_source: &'a str,
    pub geo_confidence: &'a str,
    pub network_kind: &'a str,
    pub network_risk_score: u8,
    pub network_risk_labels: &'a [String],
}

pub(crate) async fn audit_checkout_started(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: CheckoutStartedAudit<'_>,
) -> Result<(), sqlx::Error> {
    insert_billing_audit_event_tx(
        tx,
        BillingAuditEventInput {
            workspace_id: input.workspace_id,
            actor_principal_id: Some(input.actor_principal_id),
            action: "billing.checkout_started",
            target_type: "workspace",
            target_id: Some(input.workspace_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "plan_code": input.plan_code,
                "provider": input.checkout.provider.as_str(),
                "provider_route_reason": input.provider_decision.reason.as_str(),
                "provider_residency_scope": input.provider_decision.residency_scope.as_str(),
                "provider_operational_status": input.provider_decision.operational_status.as_str(),
                "provider_estimated_fee_minor": input.provider_decision.estimated_fee_minor,
                "provider_success_priority": input.provider_decision.success_priority,
                "provider_fallback_allowed": input.provider_decision.fallback_allowed,
                "provider_routing_rule_id": input.routing_rule_id,
                "provider_routing_rule_provider": input.routing_rule_provider,
                "provider_route_candidates": input.route_candidates,
                "provider_customer_id": input.checkout.provider_customer_id,
                "provider_price_id": input.checkout.provider_price_id,
                "geo_country_code": input.checkout_country,
                "price_country_code": input.checkout.price_country_code,
                "pricing_region": input.checkout.pricing_region,
                "checkout_id": input.checkout.checkout_id,
                "payment_id": input.checkout.payment_id,
                "geo": {
                    "source": input.geo_source,
                    "confidence": input.geo_confidence,
                    "network_kind": input.network_kind,
                    "risk_score": input.network_risk_score,
                    "risk_labels": input.network_risk_labels,
                    "fraud_score": input.fraud_assessment.score,
                    "fraud_decision": input.fraud_assessment.decision.as_str(),
                    "network_threat": input.fraud_assessment.network_threat.as_str(),
                    "fraud_labels": input.fraud_assessment.labels,
                    "fraud_reasons": input.fraud_assessment.reasons,
                    "enforcement_action": input.fraud_enforcement.as_str(),
                },
            }),
        },
    )
    .await
}
