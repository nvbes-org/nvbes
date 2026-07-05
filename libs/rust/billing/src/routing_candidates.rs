use serde::Serialize;

use crate::provider::ProviderCode;
use crate::routing::{
    ProviderOperationalStatus, ProviderRouteRequest, provider_estimated_fee_minor,
    provider_residency_scope, provider_success_priority,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderRouteCandidate {
    pub provider: ProviderCode,
    pub enabled: bool,
    pub operational_status: &'static str,
    pub preferred: bool,
    pub eligible: bool,
    pub exclusion_reason: Option<&'static str>,
    pub residency_scope: &'static str,
    pub estimated_fee_minor: i64,
    pub success_priority: u8,
}

pub fn provider_route_candidates(request: &ProviderRouteRequest) -> Vec<ProviderRouteCandidate> {
    let country = request.country.as_deref().map(str::to_ascii_uppercase);
    let country = country.as_deref();
    vec![
        ProviderRouteCandidate {
            provider: ProviderCode::Mollie,
            enabled: request.mollie_enabled,
            operational_status: request.mollie_status.as_str(),
            preferred: request.preferred_provider == Some(ProviderCode::Mollie),
            eligible: request.preferred_provider != Some(ProviderCode::Stripe)
                && request.mollie_enabled
                && request.currency == "EUR"
                && request.mollie_status != ProviderOperationalStatus::Unavailable,
            exclusion_reason: mollie_exclusion_reason(request),
            residency_scope: provider_residency_scope(country, ProviderCode::Mollie).as_str(),
            estimated_fee_minor: provider_estimated_fee_minor(
                ProviderCode::Mollie,
                request.amount_minor,
            ),
            success_priority: provider_success_priority(
                country,
                ProviderCode::Mollie,
                request.mollie_status,
            ),
        },
        ProviderRouteCandidate {
            provider: ProviderCode::Stripe,
            enabled: request.external_provider_fallback_enabled,
            operational_status: request.external_provider_status.as_str(),
            preferred: request.preferred_provider == Some(ProviderCode::Stripe),
            eligible: request.external_provider_fallback_enabled
                && request.external_provider_status != ProviderOperationalStatus::Unavailable,
            exclusion_reason: external_exclusion_reason(request),
            residency_scope: provider_residency_scope(country, ProviderCode::Stripe).as_str(),
            estimated_fee_minor: provider_estimated_fee_minor(
                ProviderCode::Stripe,
                request.amount_minor,
            ),
            success_priority: provider_success_priority(
                country,
                ProviderCode::Stripe,
                request.external_provider_status,
            ),
        },
    ]
}

fn mollie_exclusion_reason(request: &ProviderRouteRequest) -> Option<&'static str> {
    if request.preferred_provider == Some(ProviderCode::Stripe) {
        Some("not_preferred_by_routing_rule")
    } else if !request.mollie_enabled {
        Some("provider_disabled_or_unconfigured")
    } else if request.currency != "EUR" {
        Some("currency_not_supported")
    } else if request.mollie_status == ProviderOperationalStatus::Unavailable {
        Some("provider_unavailable")
    } else {
        None
    }
}

fn external_exclusion_reason(request: &ProviderRouteRequest) -> Option<&'static str> {
    if !request.external_provider_fallback_enabled {
        Some("external_fallback_disabled")
    } else if request.external_provider_status == ProviderOperationalStatus::Unavailable {
        Some("provider_unavailable")
    } else {
        None
    }
}
