use nvbes_billing::payments::{PaymentStatus, can_fallback_to_another_provider};
use nvbes_billing::provider::ProviderCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteRequest {
    pub country: Option<String>,
    pub currency: String,
    pub payment_method: Option<String>,
    pub amount_minor: i64,
    pub mollie_enabled: bool,
}

pub fn route_provider(request: &ProviderRouteRequest) -> ProviderCode {
    if request.mollie_enabled
        && request.currency == "EUR"
        && request
            .country
            .as_deref()
            .is_some_and(|country| matches!(country, "FR" | "BE" | "NL"))
    {
        ProviderCode::Mollie
    } else {
        ProviderCode::Stripe
    }
}

pub fn can_create_provider_fallback(status: PaymentStatus) -> bool {
    can_fallback_to_another_provider(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_is_default_provider() {
        let provider = route_provider(&ProviderRouteRequest {
            country: Some("US".to_string()),
            currency: "USD".to_string(),
            payment_method: None,
            amount_minor: 1_000,
            mollie_enabled: true,
        });
        assert_eq!(provider, ProviderCode::Stripe);
    }

    #[test]
    fn mollie_can_be_routed_for_enabled_eur_segments() {
        let provider = route_provider(&ProviderRouteRequest {
            country: Some("FR".to_string()),
            currency: "EUR".to_string(),
            payment_method: Some("card".to_string()),
            amount_minor: 1_000,
            mollie_enabled: true,
        });
        assert_eq!(provider, ProviderCode::Mollie);
    }

    #[test]
    fn fallback_is_forbidden_after_authorization() {
        assert!(!can_create_provider_fallback(PaymentStatus::Authorized));
        assert!(can_create_provider_fallback(PaymentStatus::Failed));
    }
}
