use super::super::provider_routing::{
    self, ProviderOperationalStatus, ProviderRouteDecision, ProviderRoutingError,
};
use super::super::{db, types::AuditEventInput};
use crate::http::error::AppError;
use nvbes_core::config::AppConfig;
use uuid::Uuid;

pub struct CheckoutProviderResult {
    pub provider: String,
    pub checkout_id: String,
    pub url: String,
    pub provider_customer_id: String,
    pub provider_price_id: Option<String>,
    pub price_country_code: Option<String>,
    pub pricing_region: Option<String>,
    pub payment_id: Option<String>,
    pub stripe_customer_id: String,
    pub stripe_price_id: String,
}

pub fn route_checkout_provider(
    config: &AppConfig,
    country: Option<&str>,
    amount_minor: i64,
) -> Result<ProviderRouteDecision, ProviderRoutingError> {
    provider_routing::route_provider(&provider_routing::ProviderRouteRequest {
        country: country.map(ToOwned::to_owned),
        currency: "EUR".to_string(),
        payment_method: None,
        amount_minor,
        mollie_enabled: config.billing_mollie_enabled && config.mollie_api_key.is_some(),
        mollie_status: ProviderOperationalStatus::from_config(
            &config.billing_mollie_routing_status,
        ),
        external_provider_fallback_enabled: config.billing_external_provider_fallback_enabled,
        external_provider_status: ProviderOperationalStatus::from_config(
            &config.billing_external_provider_routing_status,
        ),
    })
}

pub fn checkout_route_candidates(config: &AppConfig, amount_minor: i64) -> serde_json::Value {
    let mollie_enabled = config.billing_mollie_enabled && config.mollie_api_key.is_some();
    let mollie_status =
        ProviderOperationalStatus::from_config(&config.billing_mollie_routing_status);
    let external_status =
        ProviderOperationalStatus::from_config(&config.billing_external_provider_routing_status);

    serde_json::json!([
        {
            "provider": "mollie",
            "enabled": mollie_enabled,
            "operational_status": mollie_status.as_str(),
            "eligible": mollie_enabled && mollie_status != ProviderOperationalStatus::Unavailable,
            "exclusion_reason": mollie_exclusion_reason(mollie_enabled, mollie_status),
            "estimated_fee_minor": estimate_visible_fee_minor("mollie", amount_minor),
        },
        {
            "provider": "stripe",
            "enabled": config.billing_external_provider_fallback_enabled,
            "operational_status": external_status.as_str(),
            "eligible": config.billing_external_provider_fallback_enabled
                && external_status != ProviderOperationalStatus::Unavailable,
            "exclusion_reason": external_exclusion_reason(
                config.billing_external_provider_fallback_enabled,
                external_status,
            ),
            "estimated_fee_minor": estimate_visible_fee_minor("stripe", amount_minor),
        },
    ])
}

pub struct CheckoutRoutingBlockedAudit<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Uuid,
    pub plan_code: &'a str,
    pub routing_error: &'a str,
    pub geo_country_code: Option<&'a str>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub mollie_enabled: bool,
    pub mollie_routing_status: &'a str,
    pub external_provider_fallback_enabled: bool,
    pub external_provider_routing_status: &'a str,
    pub route_candidates: serde_json::Value,
}

pub async fn audit_checkout_routing_blocked(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: CheckoutRoutingBlockedAudit<'_>,
) -> Result<(), AppError> {
    db::insert_audit_event(
        tx,
        AuditEventInput {
            workspace_id: input.workspace_id,
            actor_user_id: Some(input.actor_user_id),
            action: "billing.checkout_routing_blocked",
            target_type: "workspace",
            target_id: Some(input.workspace_id),
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "plan_code": input.plan_code,
                "routing_error": input.routing_error,
                "geo_country_code": input.geo_country_code,
                "mollie_enabled": input.mollie_enabled,
                "mollie_routing_status": input.mollie_routing_status,
                "external_provider_fallback_enabled": input.external_provider_fallback_enabled,
                "external_provider_routing_status": input.external_provider_routing_status,
                "route_candidates": input.route_candidates,
            }),
        },
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "Audit payload mirrors checkout request and routing policy context."
)]
pub fn checkout_routing_blocked_audit<'a>(
    config: &'a AppConfig,
    workspace_id: Uuid,
    actor_user_id: Uuid,
    plan_code: &'a str,
    amount_minor: i64,
    routing_error: &'a str,
    geo_country_code: Option<&'a str>,
    ip: Option<&'a str>,
    user_agent: Option<&'a str>,
) -> CheckoutRoutingBlockedAudit<'a> {
    CheckoutRoutingBlockedAudit {
        workspace_id,
        actor_user_id,
        plan_code,
        routing_error,
        geo_country_code,
        ip,
        user_agent,
        mollie_enabled: config.billing_mollie_enabled && config.mollie_api_key.is_some(),
        mollie_routing_status: &config.billing_mollie_routing_status,
        external_provider_fallback_enabled: config.billing_external_provider_fallback_enabled,
        external_provider_routing_status: &config.billing_external_provider_routing_status,
        route_candidates: checkout_route_candidates(config, amount_minor),
    }
}

pub fn checkout_routing_blocked_error(error: ProviderRoutingError) -> AppError {
    AppError::conflict(
        error.as_str(),
        "No compliant billing provider is available for this checkout policy.",
    )
}

fn mollie_exclusion_reason(
    enabled: bool,
    status: ProviderOperationalStatus,
) -> Option<&'static str> {
    if !enabled {
        Some("provider_disabled_or_unconfigured")
    } else if status == ProviderOperationalStatus::Unavailable {
        Some("provider_unavailable")
    } else {
        None
    }
}

fn external_exclusion_reason(
    enabled: bool,
    status: ProviderOperationalStatus,
) -> Option<&'static str> {
    if !enabled {
        Some("external_fallback_disabled")
    } else if status == ProviderOperationalStatus::Unavailable {
        Some("provider_unavailable")
    } else {
        None
    }
}

fn estimate_visible_fee_minor(provider: &str, amount_minor: i64) -> i64 {
    let amount_minor = amount_minor.max(0);
    match provider {
        "mollie" => 25 + amount_minor * 12 / 1_000,
        _ => 25 + amount_minor * 15 / 1_000,
    }
}
