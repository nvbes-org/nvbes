use crate::payments::PaymentStatus;
use crate::provider::ProviderCode;
use crate::routing::{
    ProviderOperationalStatus, ProviderResidencyScope, ProviderRouteReason, ProviderRouteRequest,
    ProviderRoutingError, can_create_provider_fallback, provider_estimated_fee_minor,
    provider_residency_scope, provider_success_priority, route_provider,
};
use crate::routing_candidates::provider_route_candidates;

#[test]
fn stripe_is_not_default_when_external_fallback_is_disabled() {
    let error = route_provider(&ProviderRouteRequest {
        country: Some("US".to_string()),
        currency: "USD".to_string(),
        payment_method: None,
        amount_minor: 1_000,
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
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
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
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
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
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
        preferred_provider: None,
        mollie_enabled: false,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: true,
        external_provider_status: ProviderOperationalStatus::Available,
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
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
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

#[test]
fn unavailable_regional_provider_is_excluded() {
    let error = route_provider(&ProviderRouteRequest {
        country: Some("FR".to_string()),
        currency: "EUR".to_string(),
        payment_method: Some("card".to_string()),
        amount_minor: 1_000,
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Unavailable,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
    });
    assert_eq!(error, Err(ProviderRoutingError::NoCompliantProvider));
}

#[test]
fn degraded_provider_remains_usable_when_it_is_the_only_compliant_provider() {
    let decision = route_provider(&ProviderRouteRequest {
        country: Some("FR".to_string()),
        currency: "EUR".to_string(),
        payment_method: Some("card".to_string()),
        amount_minor: 1_000,
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Degraded,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
    })
    .expect("degraded regional provider should remain usable");
    assert_eq!(decision.provider, ProviderCode::Mollie);
    assert_eq!(
        decision.operational_status,
        ProviderOperationalStatus::Degraded
    );
    assert!(decision.success_priority < 95);
}

#[test]
fn preferred_external_provider_can_override_local_provider_when_explicitly_enabled() {
    let decision = route_provider(&ProviderRouteRequest {
        country: Some("FR".to_string()),
        currency: "EUR".to_string(),
        payment_method: Some("card".to_string()),
        amount_minor: 1_000,
        preferred_provider: Some(ProviderCode::Stripe),
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: true,
        external_provider_status: ProviderOperationalStatus::Available,
    })
    .expect("explicit external provider should be selected");
    assert_eq!(decision.provider, ProviderCode::Stripe);
    assert_eq!(
        decision.reason,
        ProviderRouteReason::ExternalProviderFallback
    );
}

#[test]
fn preferred_regional_provider_can_fallback_to_external_when_unavailable() {
    let decision = route_provider(&ProviderRouteRequest {
        country: Some("FR".to_string()),
        currency: "EUR".to_string(),
        payment_method: Some("card".to_string()),
        amount_minor: 1_000,
        preferred_provider: Some(ProviderCode::Mollie),
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Unavailable,
        external_provider_fallback_enabled: true,
        external_provider_status: ProviderOperationalStatus::Available,
    })
    .expect("external fallback should be selected when preferred regional provider is down");
    assert_eq!(decision.provider, ProviderCode::Stripe);
}

#[test]
fn route_candidates_explain_eligibility_and_visible_fees() {
    let candidates = provider_route_candidates(&ProviderRouteRequest {
        country: Some("FR".to_string()),
        currency: "EUR".to_string(),
        payment_method: Some("card".to_string()),
        amount_minor: 10_000,
        preferred_provider: Some(ProviderCode::Mollie),
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: false,
        external_provider_status: ProviderOperationalStatus::Available,
    });

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].provider, ProviderCode::Mollie);
    assert!(candidates[0].eligible);
    assert_eq!(candidates[0].residency_scope, "local");
    assert_eq!(candidates[0].success_priority, 95);
    assert_eq!(candidates[0].estimated_fee_minor, 145);
    assert_eq!(candidates[0].exclusion_reason, None);

    assert_eq!(candidates[1].provider, ProviderCode::Stripe);
    assert!(!candidates[1].eligible);
    assert_eq!(candidates[1].residency_scope, "external");
    assert_eq!(candidates[1].success_priority, 80);
    assert_eq!(
        candidates[1].exclusion_reason,
        Some("external_fallback_disabled")
    );
    assert_eq!(candidates[1].estimated_fee_minor, 175);
}

#[test]
fn selected_provider_is_reported_as_eligible_candidate() {
    let request = ProviderRouteRequest {
        country: Some("FR".to_string()),
        currency: "EUR".to_string(),
        payment_method: Some("card".to_string()),
        amount_minor: 10_000,
        preferred_provider: None,
        mollie_enabled: true,
        mollie_status: ProviderOperationalStatus::Available,
        external_provider_fallback_enabled: true,
        external_provider_status: ProviderOperationalStatus::Available,
    };

    let decision = route_provider(&request).expect("provider should be selected");
    let candidates = provider_route_candidates(&request);

    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.provider == decision.provider && candidate.eligible),
        "selected provider must be listed as eligible in route candidates"
    );
}

#[test]
fn cb_provider_is_modelled_as_local_for_french_card_routing() {
    assert_eq!(
        provider_residency_scope(Some("FR"), ProviderCode::Cb),
        ProviderResidencyScope::Local
    );
    assert_eq!(
        provider_residency_scope(Some("DE"), ProviderCode::Cb),
        ProviderResidencyScope::Regional
    );
    assert_eq!(provider_estimated_fee_minor(ProviderCode::Cb, 10_000), 125);
    assert_eq!(
        provider_success_priority(
            Some("FR"),
            ProviderCode::Cb,
            ProviderOperationalStatus::Available
        ),
        96
    );
}
