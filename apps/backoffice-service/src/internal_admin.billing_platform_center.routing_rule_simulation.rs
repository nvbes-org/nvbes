use serde::Serialize;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1::SimulateAdminBillingRoutingRequest;

#[derive(Debug, Clone)]
pub(crate) struct RoutingRuleSimulationInput {
    pub(crate) country: Option<String>,
    pub(crate) currency: String,
    pub(crate) payment_method: String,
    pub(crate) customer_type: String,
    pub(crate) amount_minor: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct RoutingRuleSimulationResult {
    pub(crate) input: NormalizedRoutingRuleSimulationInput,
    pub(crate) matched_rule: Option<MatchedRoutingRule>,
    pub(crate) outcome: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRoutingRuleSimulationInput {
    pub(crate) country: Option<String>,
    pub(crate) currency: String,
    pub(crate) payment_method: String,
    pub(crate) customer_type: String,
    pub(crate) amount_minor: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatchedRoutingRule {
    pub(crate) id: Uuid,
    pub(crate) priority: i32,
    pub(crate) provider: String,
    pub(crate) fallback_enabled: bool,
}

pub(crate) async fn simulate_routing_rule(
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: RoutingRuleSimulationInput,
) -> Result<RoutingRuleSimulationResult, AppError> {
    let result = crate::billing_grpc::simulate_admin_billing_routing(
        billing_grpc_endpoint,
        access,
        workspace_id,
        SimulateAdminBillingRoutingRequest {
            context: None,
            workspace_id: String::new(),
            country: input.country.unwrap_or_default(),
            currency: input.currency,
            payment_method: input.payment_method,
            customer_type: input.customer_type,
            amount_minor: input.amount_minor,
        },
    )
    .await?;
    let normalized = result
        .input
        .ok_or_else(|| AppError::internal("billing_grpc_decode", "missing routing input"))?;
    Ok(RoutingRuleSimulationResult {
        input: NormalizedRoutingRuleSimulationInput {
            country: empty_to_none(normalized.country),
            currency: normalized.currency,
            payment_method: normalized.payment_method,
            customer_type: normalized.customer_type,
            amount_minor: normalized.amount_minor,
        },
        matched_rule: result
            .matched_rule
            .map(matched_rule_from_grpc)
            .transpose()?,
        outcome: result.outcome,
    })
}

fn matched_rule_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::AdminBillingRoutingMatchedRule,
) -> Result<MatchedRoutingRule, AppError> {
    Ok(MatchedRoutingRule {
        id: Uuid::parse_str(&value.id)
            .map_err(|_| AppError::internal("billing_grpc_decode", "matched routing rule id"))?,
        priority: value.priority,
        provider: value.provider,
        fallback_enabled: value.fallback_enabled,
    })
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
