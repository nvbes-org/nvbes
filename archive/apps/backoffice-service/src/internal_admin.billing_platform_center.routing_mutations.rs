use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_grpc_platform::{
    BackofficeBillingPlatformActionKind, run_billing_platform_action,
};
use crate::billing_platform_center_mutations::{PlatformTarget, record_platform_action};
use crate::billing_platform_center_routing_rule_validation::validate_create_routing_rule;
use crate::billing_platform_center_types::BillingPlatformActionResult;
use crate::billing_platform_center_validation::validate_reason;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1::CreateBillingRoutingRuleInput as GrpcCreateRoutingRuleInput;

pub(crate) struct CreateRoutingRuleInput {
    pub(crate) provider: String,
    pub(crate) country: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) payment_method: Option<String>,
    pub(crate) customer_type: Option<String>,
    pub(crate) min_amount_minor: Option<i64>,
    pub(crate) max_amount_minor: Option<i64>,
    pub(crate) fallback_enabled: bool,
    pub(crate) priority: i32,
    pub(crate) reason: String,
}

pub(crate) async fn create_routing_rule(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: CreateRoutingRuleInput,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_create_routing_rule(&input)?;
    let reason = input.reason.clone();
    let outcome = run_billing_platform_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingPlatformActionKind::CreateRoutingRule,
        None,
        reason.clone(),
        Some(routing_rule_input(input)),
    )
    .await?;
    let rule_id = outcome.object_id;
    record_platform_action(
        db,
        access,
        workspace_id,
        reason,
        PlatformTarget::RoutingRule(rule_id),
        outcome,
    )
    .await
}

pub(crate) async fn enable_routing_rule(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rule_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    transition_routing_rule(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        rule_id,
        BackofficeBillingPlatformActionKind::EnableRoutingRule,
        reason,
    )
    .await
}

pub(crate) async fn disable_routing_rule(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rule_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    transition_routing_rule(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        rule_id,
        BackofficeBillingPlatformActionKind::DisableRoutingRule,
        reason,
    )
    .await
}

async fn transition_routing_rule(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rule_id: Uuid,
    action_kind: BackofficeBillingPlatformActionKind,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_reason(&reason)?;
    let outcome = run_billing_platform_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        action_kind,
        Some(rule_id),
        reason.clone(),
        None,
    )
    .await?;
    record_platform_action(
        db,
        access,
        workspace_id,
        reason,
        PlatformTarget::RoutingRule(rule_id),
        outcome,
    )
    .await
}

fn routing_rule_input(input: CreateRoutingRuleInput) -> GrpcCreateRoutingRuleInput {
    let min_amount_minor = input.min_amount_minor.unwrap_or_default();
    let max_amount_minor = input.max_amount_minor.unwrap_or_default();
    GrpcCreateRoutingRuleInput {
        provider: input.provider,
        country: input.country.unwrap_or_default(),
        currency: input.currency.unwrap_or_default(),
        payment_method: input.payment_method.unwrap_or_default(),
        customer_type: input.customer_type.unwrap_or_default(),
        min_amount_minor,
        max_amount_minor,
        has_min_amount_minor: min_amount_minor != 0 || input.min_amount_minor.is_some(),
        has_max_amount_minor: max_amount_minor != 0 || input.max_amount_minor.is_some(),
        fallback_enabled: input.fallback_enabled,
        priority: input.priority,
    }
}
