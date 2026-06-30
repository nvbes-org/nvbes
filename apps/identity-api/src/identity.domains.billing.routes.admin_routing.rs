use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::billing::service::{
        DisableProviderRoutingRuleInput, ProviderRoutingRuleInput,
        ProviderRoutingRuleMutationResult, create_provider_routing_rule,
        disable_provider_routing_rule,
    },
    http::error::AppError,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/internal/billing/provider-routing-rules",
            post(create_provider_routing_rule_route),
        )
        .route(
            "/internal/billing/provider-routing-rules/{routingRuleId}/disable",
            post(disable_provider_routing_rule_route),
        )
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreateProviderRoutingRuleRequest {
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    provider: String,
    country: Option<String>,
    currency: Option<String>,
    payment_method: Option<String>,
    customer_type: Option<String>,
    min_amount_minor: Option<i64>,
    max_amount_minor: Option<i64>,
    fallback_enabled: bool,
    priority: i32,
    reason: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct DisableProviderRoutingRuleRequest {
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    reason: String,
}

pub(crate) async fn create_provider_routing_rule_route(
    State(state): State<AppState>,
    Json(request): Json<CreateProviderRoutingRuleRequest>,
) -> Result<Json<ProviderRoutingRuleMutationResult>, AppError> {
    let result = create_provider_routing_rule(
        &state.db,
        ProviderRoutingRuleInput {
            tenant_id: request.tenant_id,
            workspace_id: request.workspace_id,
            actor_principal_id: request.actor_principal_id,
            provider: request.provider,
            country: request.country,
            currency: request.currency,
            payment_method: request.payment_method,
            customer_type: request.customer_type,
            min_amount_minor: request.min_amount_minor,
            max_amount_minor: request.max_amount_minor,
            fallback_enabled: request.fallback_enabled,
            priority: request.priority,
            reason: request.reason,
        },
    )
    .await?;
    Ok(Json(result))
}

pub(crate) async fn disable_provider_routing_rule_route(
    State(state): State<AppState>,
    Path(routing_rule_id): Path<Uuid>,
    Json(request): Json<DisableProviderRoutingRuleRequest>,
) -> Result<Json<ProviderRoutingRuleMutationResult>, AppError> {
    let result = disable_provider_routing_rule(
        &state.db,
        DisableProviderRoutingRuleInput {
            tenant_id: request.tenant_id,
            workspace_id: request.workspace_id,
            actor_principal_id: request.actor_principal_id,
            routing_rule_id,
            reason: request.reason,
        },
    )
    .await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_admin_routing_routes_are_registered() {
        let router = router();
        let _ = router;
    }
}
