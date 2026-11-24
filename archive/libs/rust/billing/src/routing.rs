use crate::payments::{PaymentStatus, can_fallback_to_another_provider};
use crate::provider::ProviderCode;

const LOCAL_COUNTRIES: &[&str] = &["FR"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteRequest {
    pub country: Option<String>,
    pub currency: String,
    pub payment_method: Option<String>,
    pub amount_minor: i64,
    pub preferred_provider: Option<ProviderCode>,
    pub mollie_enabled: bool,
    pub mollie_status: ProviderOperationalStatus,
    pub external_provider_fallback_enabled: bool,
    pub external_provider_status: ProviderOperationalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteDecision {
    pub provider: ProviderCode,
    pub reason: ProviderRouteReason,
    pub residency_scope: ProviderResidencyScope,
    pub estimated_fee_minor: i64,
    pub success_priority: u8,
    pub fallback_allowed: bool,
    pub operational_status: ProviderOperationalStatus,
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
pub enum ProviderOperationalStatus {
    Available,
    Degraded,
    Unavailable,
}

impl ProviderOperationalStatus {
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "available" => ProviderOperationalStatus::Available,
            "degraded" => ProviderOperationalStatus::Degraded,
            _ => ProviderOperationalStatus::Unavailable,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderOperationalStatus::Available => "available",
            ProviderOperationalStatus::Degraded => "degraded",
            ProviderOperationalStatus::Unavailable => "unavailable",
        }
    }

    pub fn accepts_new_checkouts(self) -> bool {
        self != ProviderOperationalStatus::Unavailable
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProviderCandidate {
    provider: ProviderCode,
    residency_scope: ProviderResidencyScope,
    estimated_fee_minor: i64,
    success_priority: u8,
    operational_status: ProviderOperationalStatus,
}

pub fn route_provider(
    request: &ProviderRouteRequest,
) -> Result<ProviderRouteDecision, ProviderRoutingError> {
    let country = request.country.as_deref().map(str::to_ascii_uppercase);
    let country = country.as_deref();
    let mut candidates = Vec::new();

    if provider_matches_preference(request.preferred_provider, ProviderCode::Mollie)
        && request.mollie_enabled
        && request.currency == "EUR"
        && request.mollie_status.accepts_new_checkouts()
    {
        candidates.push(ProviderCandidate {
            provider: ProviderCode::Mollie,
            residency_scope: provider_residency_scope(country, ProviderCode::Mollie),
            estimated_fee_minor: provider_estimated_fee_minor(
                ProviderCode::Mollie,
                request.amount_minor,
            ),
            success_priority: provider_success_priority(
                country,
                ProviderCode::Mollie,
                request.mollie_status,
            ),
            operational_status: request.mollie_status,
        });
    }

    if request.external_provider_fallback_enabled
        && request.external_provider_status.accepts_new_checkouts()
    {
        candidates.push(ProviderCandidate {
            provider: ProviderCode::Stripe,
            residency_scope: provider_residency_scope(country, ProviderCode::Stripe),
            estimated_fee_minor: provider_estimated_fee_minor(
                ProviderCode::Stripe,
                request.amount_minor,
            ),
            success_priority: provider_success_priority(
                country,
                ProviderCode::Stripe,
                request.external_provider_status,
            ),
            operational_status: request.external_provider_status,
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
        operational_status: selected.operational_status,
    })
}

pub fn can_create_provider_fallback(status: PaymentStatus) -> bool {
    can_fallback_to_another_provider(status)
}

fn provider_matches_preference(preferred: Option<ProviderCode>, provider: ProviderCode) -> bool {
    preferred.is_none_or(|preferred| preferred == provider)
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

pub fn provider_residency_scope(
    country: Option<&str>,
    provider: ProviderCode,
) -> ProviderResidencyScope {
    match provider {
        ProviderCode::Mollie if country.is_some_and(|value| LOCAL_COUNTRIES.contains(&value)) => {
            ProviderResidencyScope::Local
        }
        ProviderCode::Mollie => ProviderResidencyScope::Regional,
        ProviderCode::Cb if country.is_some_and(|value| value == "FR") => {
            ProviderResidencyScope::Local
        }
        ProviderCode::Cb => ProviderResidencyScope::Regional,
        ProviderCode::Stripe => ProviderResidencyScope::External,
    }
}

fn residency_rank(scope: ProviderResidencyScope) -> u8 {
    match scope {
        ProviderResidencyScope::Local => 0,
        ProviderResidencyScope::Regional => 1,
        ProviderResidencyScope::External => 2,
    }
}

pub fn provider_estimated_fee_minor(provider: ProviderCode, amount_minor: i64) -> i64 {
    let amount_minor = amount_minor.max(0);
    match provider {
        ProviderCode::Cb => 25 + amount_minor * 10 / 1_000,
        ProviderCode::Mollie => 25 + amount_minor * 12 / 1_000,
        ProviderCode::Stripe => 25 + amount_minor * 15 / 1_000,
    }
}

pub fn provider_success_priority(
    country: Option<&str>,
    provider: ProviderCode,
    status: ProviderOperationalStatus,
) -> u8 {
    let base = match provider {
        ProviderCode::Cb if country.is_some_and(|value| value == "FR") => 96,
        ProviderCode::Cb => 86,
        ProviderCode::Mollie
            if country.is_some_and(|value| matches!(value, "FR" | "BE" | "NL")) =>
        {
            95
        }
        ProviderCode::Mollie => 85,
        ProviderCode::Stripe => 80,
    };

    match status {
        ProviderOperationalStatus::Available => base,
        ProviderOperationalStatus::Degraded => base.saturating_sub(20),
        ProviderOperationalStatus::Unavailable => 0,
    }
}
