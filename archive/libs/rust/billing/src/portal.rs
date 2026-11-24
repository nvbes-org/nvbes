use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::provider::{ProviderCode, provider_code, provider_supports_external_portal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodUpdateFlow {
    NvbesProviderRedirect,
    ProviderPortalUnavailable,
}

impl PaymentMethodUpdateFlow {
    pub fn for_provider(provider: ProviderCode) -> Self {
        if provider_supports_external_portal(provider) {
            Self::NvbesProviderRedirect
        } else {
            Self::ProviderPortalUnavailable
        }
    }

    pub fn for_provider_code(value: &str) -> Self {
        provider_code(value).map_or(Self::ProviderPortalUnavailable, Self::for_provider)
    }
}

#[cfg(test)]
mod tests {
    use super::{PaymentMethodUpdateFlow, ProviderCode};

    #[test]
    fn flow_matches_provider_capabilities() {
        assert_eq!(
            PaymentMethodUpdateFlow::for_provider(ProviderCode::Stripe),
            PaymentMethodUpdateFlow::NvbesProviderRedirect
        );
        assert_eq!(
            PaymentMethodUpdateFlow::for_provider(ProviderCode::Mollie),
            PaymentMethodUpdateFlow::ProviderPortalUnavailable
        );
    }

    #[test]
    fn unsupported_provider_code_returns_unavailable_flow() {
        assert_eq!(
            PaymentMethodUpdateFlow::for_provider_code("unknown"),
            PaymentMethodUpdateFlow::ProviderPortalUnavailable
        );
    }

    #[test]
    fn flow_serializes_as_public_code() {
        assert_eq!(
            serde_json::to_value(PaymentMethodUpdateFlow::NvbesProviderRedirect)
                .expect("flow should serialize"),
            serde_json::json!("nvbes_provider_redirect")
        );
    }
}
