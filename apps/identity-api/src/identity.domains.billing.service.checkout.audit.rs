use super::super::{db, provider_routing, types::AuditEventInput};
use super::checkout_routing::CheckoutProviderResult;
use crate::http::error::AppError;
use uuid::Uuid;

pub struct CheckoutStartedAudit<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Uuid,
    pub plan_code: &'a str,
    pub checkout_country: Option<&'a str>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub provider_decision: &'a provider_routing::ProviderRouteDecision,
    pub route_candidates: serde_json::Value,
    pub routing_rule_id: Option<Uuid>,
    pub routing_rule_provider: Option<&'a str>,
    pub checkout: &'a CheckoutProviderResult,
    pub geo_metadata: serde_json::Value,
}

pub async fn audit_checkout_started(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: CheckoutStartedAudit<'_>,
) -> Result<(), AppError> {
    db::insert_audit_event(
        tx,
        AuditEventInput {
            workspace_id: input.workspace_id,
            actor_user_id: Some(input.actor_user_id),
            action: "billing.checkout_started",
            target_type: "workspace",
            target_id: Some(input.workspace_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "plan_code": input.plan_code,
                "provider": input.checkout.provider,
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
                "geo": input.geo_metadata,
                "price_country_code": input.checkout.price_country_code,
                "pricing_region": input.checkout.pricing_region,
                "checkout_id": input.checkout.checkout_id,
                "payment_id": input.checkout.payment_id,
            }),
        },
    )
    .await
}
