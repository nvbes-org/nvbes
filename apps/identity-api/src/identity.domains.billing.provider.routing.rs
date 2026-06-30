use nvbes_billing::payments::{PaymentStatus, can_fallback_to_another_provider};
use nvbes_billing::provider::ProviderCode;

const LOCAL_COUNTRIES: &[&str] = &["FR"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteRequest {
    pub country: Option<String>,
    pub currency: String,
    pub payment_method: Option<String>,
    pub amount_minor: i64,
    pub mollie_enabled: bool,
    pub external_provider_fallback_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteDecision {
    pub provider: ProviderCode,
    pub reason: ProviderRouteReason,
    pub residency_scope: ProviderResidencyScope,
    pub estimated_fee_minor: i64,
    pub success_priority: u8,
    pub fallback_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteReason {
    LocalResidencyPreferred,
    RegionalResidencyPreferred,
    LowestEstimatedCost,
    ExternalProviderFallback,
}

impl ProviderRouteReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderRouteReason::LocalResidencyPreferred => "local_residency_preferred",
            ProviderRouteReason::RegionalResidencyPreferred => "regional_residency_preferred",
            ProviderRouteReason::LowestEstimatedCost => "lowest_estimated_cost",
            ProviderRouteReason::ExternalProviderFallback => "external_provider_fallback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderResidencyScope {
    Local,
    Regional,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRoutingError {
    NoCompliantProvider,
}

impl ProviderRoutingError {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderRoutingError::NoCompliantProvider => "no_compliant_billing_provider",
        }
    }
}

impl ProviderResidencyScope {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderResidencyScope::Local => "local",
            ProviderResidencyScope::Regional => "regional",
            ProviderResidencyScope::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProviderCandidate {
    provider: ProviderCode,
    residency_scope: ProviderResidencyScope,
    estimated_fee_minor: i64,
    success_priority: u8,
}

pub fn route_provider(
    request: &ProviderRouteRequest,
) -> Result<ProviderRouteDecision, ProviderRoutingError> {
    let country = request.country.as_deref().map(str::to_ascii_uppercase);
    let country = country.as_deref();
    let mut candidates = Vec::new();

    if request.mollie_enabled && request.currency == "EUR" {
        candidates.push(ProviderCandidate {
            provider: ProviderCode::Mollie,
            residency_scope: residency_scope(country, ProviderCode::Mollie),
            estimated_fee_minor: estimate_fee_minor(ProviderCode::Mollie, request.amount_minor),
            success_priority: success_priority(country, ProviderCode::Mollie),
        });
    }

    if request.external_provider_fallback_enabled {
        candidates.push(ProviderCandidate {
            provider: ProviderCode::Stripe,
            residency_scope: residency_scope(country, ProviderCode::Stripe),
            estimated_fee_minor: estimate_fee_minor(ProviderCode::Stripe, request.amount_minor),
            success_priority: success_priority(country, ProviderCode::Stripe),
        });
    }

    let selected = candidates
        .into_iter()
        .min_by_key(|candidate| {
            (
                residency_rank(candidate.residency_scope),
                std::cmp::Reverse(candidate.success_priority),
                candidate.estimated_fee_minor,
            )
        })
        .ok_or(ProviderRoutingError::NoCompliantProvider)?;

    Ok(ProviderRouteDecision {
        provider: selected.provider,
        reason: route_reason(request, selected),
        residency_scope: selected.residency_scope,
        estimated_fee_minor: selected.estimated_fee_minor,
        success_priority: selected.success_priority,
        fallback_allowed: true,
    })
}

pub fn can_create_provider_fallback(status: PaymentStatus) -> bool {
    can_fallback_to_another_provider(status)
}

fn route_reason(
    request: &ProviderRouteRequest,
    selected: ProviderCandidate,
) -> ProviderRouteReason {
    if selected.provider == ProviderCode::Stripe
        && request.mollie_enabled
        && selected.residency_scope != ProviderResidencyScope::External
    {
        return ProviderRouteReason::LowestEstimatedCost;
    }

    match selected.residency_scope {
        ProviderResidencyScope::Local => ProviderRouteReason::LocalResidencyPreferred,
        ProviderResidencyScope::Regional => ProviderRouteReason::RegionalResidencyPreferred,
        ProviderResidencyScope::External => ProviderRouteReason::ExternalProviderFallback,
    }
}

fn residency_scope(country: Option<&str>, provider: ProviderCode) -> ProviderResidencyScope {
    match provider {
        ProviderCode::Mollie if country.is_some_and(|value| LOCAL_COUNTRIES.contains(&value)) => {
            ProviderResidencyScope::Local
        }
        ProviderCode::Mollie => ProviderResidencyScope::Regional,
        _ => ProviderResidencyScope::External,
    }
}

fn residency_rank(scope: ProviderResidencyScope) -> u8 {
    match scope {
        ProviderResidencyScope::Local => 0,
        ProviderResidencyScope::Regional => 1,
        ProviderResidencyScope::External => 2,
    }
}

fn estimate_fee_minor(provider: ProviderCode, amount_minor: i64) -> i64 {
    let amount_minor = amount_minor.max(0);
    match provider {
        ProviderCode::Mollie => 25 + amount_minor * 12 / 1_000,
        ProviderCode::Stripe => 25 + amount_minor * 15 / 1_000,
    }
}

fn success_priority(country: Option<&str>, provider: ProviderCode) -> u8 {
    match provider {
        ProviderCode::Mollie
            if country.is_some_and(|value| matches!(value, "FR" | "BE" | "NL")) =>
        {
            95
        }
        ProviderCode::Mollie => 85,
        ProviderCode::Stripe => 80,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_is_default_provider() {
        let error = route_provider(&ProviderRouteRequest {
            country: Some("US".to_string()),
            currency: "USD".to_string(),
            payment_method: None,
            amount_minor: 1_000,
            mollie_enabled: true,
            external_provider_fallback_enabled: false,
        });
        assert_eq!(error, Err(ProviderRoutingError::NoCompliantProvider));
    }

    #[test]
    fn local_provider_wins_for_french_eur_checkout() {
        let decision = route_provider(&ProviderRouteRequest {
            country: Some("FR".to_string()),
            currency: "EUR".to_string(),
            payment_method: Some("card".to_string()),
            amount_minor: 1_000,
            mollie_enabled: true,
            external_provider_fallback_enabled: false,
        })
        .expect("local provider should be selected");
        assert_eq!(decision.provider, ProviderCode::Mollie);
        assert_eq!(
            decision.reason,
            ProviderRouteReason::LocalResidencyPreferred
        );
        assert_eq!(decision.residency_scope, ProviderResidencyScope::Local);
    }

    #[test]
    fn regional_provider_wins_for_eu_eur_checkout() {
        let decision = route_provider(&ProviderRouteRequest {
            country: Some("DE".to_string()),
            currency: "EUR".to_string(),
            payment_method: Some("card".to_string()),
            amount_minor: 1_000,
            mollie_enabled: true,
            external_provider_fallback_enabled: false,
        })
        .expect("regional provider should be selected");
        assert_eq!(decision.provider, ProviderCode::Mollie);
        assert_eq!(
            decision.reason,
            ProviderRouteReason::RegionalResidencyPreferred
        );
        assert_eq!(decision.residency_scope, ProviderResidencyScope::Regional);
    }

    #[test]
    fn external_provider_is_only_used_when_fallback_is_enabled() {
        let decision = route_provider(&ProviderRouteRequest {
            country: Some("FR".to_string()),
            currency: "EUR".to_string(),
            payment_method: Some("card".to_string()),
            amount_minor: 1_000,
            mollie_enabled: false,
            external_provider_fallback_enabled: true,
        })
        .expect("external fallback should be selected");
        assert_eq!(decision.provider, ProviderCode::Stripe);
        assert_eq!(
            decision.reason,
            ProviderRouteReason::ExternalProviderFallback
        );
        assert_eq!(decision.residency_scope, ProviderResidencyScope::External);
    }

    #[test]
    fn european_provider_wins_for_eur_even_when_customer_is_outside_europe() {
        let decision = route_provider(&ProviderRouteRequest {
            country: Some("US".to_string()),
            currency: "EUR".to_string(),
            payment_method: Some("card".to_string()),
            amount_minor: 1_000,
            mollie_enabled: true,
            external_provider_fallback_enabled: false,
        })
        .expect("regional provider should be selected");
        assert_eq!(decision.provider, ProviderCode::Mollie);
        assert_eq!(
            decision.reason,
            ProviderRouteReason::RegionalResidencyPreferred
        );
        assert_eq!(decision.residency_scope, ProviderResidencyScope::Regional);
    }

    #[test]
    fn fallback_is_forbidden_after_authorization() {
        assert!(!can_create_provider_fallback(PaymentStatus::Authorized));
        assert!(can_create_provider_fallback(PaymentStatus::Failed));
    }
}
