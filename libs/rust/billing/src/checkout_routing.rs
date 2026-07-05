use crate::db::ProviderRoutingRule;
use crate::provider::ProviderCode;
use crate::{
    ProviderOperationalStatus, ProviderRouteDecision, ProviderRouteRequest, ProviderRoutingError,
    provider_route_candidates, route_provider,
};
use nvbes_core::config::AppConfig;

pub struct CheckoutProviderResult {
    pub provider: ProviderCode,
    pub checkout_id: String,
    pub url: String,
    pub provider_customer_id: String,
    pub provider_product_id: Option<String>,
    pub provider_price_id: Option<String>,
    pub price_country_code: Option<String>,
    pub pricing_region: Option<String>,
    pub payment_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub stripe_price_id: Option<String>,
}

pub fn route_checkout_provider(
    config: &AppConfig,
    country: Option<&str>,
    amount_minor: i64,
    rule: Option<&ProviderRoutingRule>,
) -> Result<ProviderRouteDecision, ProviderRoutingError> {
    route_provider(&checkout_route_request(config, country, amount_minor, rule))
}

pub fn checkout_route_candidates(
    config: &AppConfig,
    country: Option<&str>,
    amount_minor: i64,
    rule: Option<&ProviderRoutingRule>,
) -> serde_json::Value {
    let request = checkout_route_request(config, country, amount_minor, rule);
    serde_json::to_value(provider_route_candidates(&request))
        .expect("provider route candidates should serialize")
}

pub fn checkout_route_request(
    config: &AppConfig,
    country: Option<&str>,
    amount_minor: i64,
    rule: Option<&ProviderRoutingRule>,
) -> ProviderRouteRequest {
    ProviderRouteRequest {
        country: country.map(ToOwned::to_owned),
        currency: "EUR".to_string(),
        payment_method: None,
        amount_minor,
        preferred_provider: rule.map(|rule| rule.provider),
        mollie_enabled: config.billing_mollie_enabled && config.mollie_api_key.is_some(),
        mollie_status: ProviderOperationalStatus::from_config(
            &config.billing_mollie_routing_status,
        ),
        external_provider_fallback_enabled: external_fallback_enabled(config, rule),
        external_provider_status: ProviderOperationalStatus::from_config(
            &config.billing_external_provider_routing_status,
        ),
    }
}

pub fn external_fallback_enabled(config: &AppConfig, rule: Option<&ProviderRoutingRule>) -> bool {
    config.billing_external_provider_fallback_enabled
        || rule.is_some_and(|rule| rule.fallback_enabled || rule.provider == ProviderCode::Stripe)
}
