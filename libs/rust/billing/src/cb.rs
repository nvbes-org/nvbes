use crate::provider::ProviderCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CbServiceKind {
    SafeR,
    UpdatR,
    FastR,
}

impl CbServiceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CbServiceKind::SafeR => "safe_r",
            CbServiceKind::UpdatR => "updat_r",
            CbServiceKind::FastR => "fast_r",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CbPaymentContext {
    pub customer_initiated: bool,
    pub recurring: bool,
    pub stored_credential: bool,
    pub card_country: Option<String>,
    pub merchant_country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CbCapabilityDecision {
    pub provider: ProviderCode,
    pub service: CbServiceKind,
    pub eligible: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CbIntegrationPort {
    pub acquirer_reference: Option<String>,
    pub pat_reference: Option<String>,
    pub enabled_services: Vec<CbServiceKind>,
}

pub fn cb_service_codes() -> &'static [&'static str] {
    &["safe_r", "updat_r", "fast_r"]
}

pub fn evaluate_cb_capability(
    service: CbServiceKind,
    context: &CbPaymentContext,
) -> CbCapabilityDecision {
    let eligible = match service {
        CbServiceKind::SafeR => {
            context.customer_initiated && !context.recurring && cb_card_context(context)
        }
        CbServiceKind::UpdatR => context.stored_credential && cb_card_context(context),
        CbServiceKind::FastR => context.customer_initiated && cb_card_context(context),
    };
    CbCapabilityDecision {
        provider: ProviderCode::Cb,
        service,
        eligible,
        reason: cb_capability_reason(service, context, eligible),
    }
}

pub fn validate_cb_integration_port(port: &CbIntegrationPort) -> Result<(), &'static str> {
    if port
        .acquirer_reference
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
        && port
            .pat_reference
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err("cb_acquirer_or_pat_required");
    }
    if port.enabled_services.is_empty() {
        return Err("cb_service_required");
    }
    Ok(())
}

fn cb_card_context(context: &CbPaymentContext) -> bool {
    context
        .card_country
        .as_deref()
        .is_some_and(|country| country.eq_ignore_ascii_case("FR"))
        && context
            .merchant_country
            .as_deref()
            .is_none_or(|country| country.eq_ignore_ascii_case("FR"))
}

fn cb_capability_reason(
    service: CbServiceKind,
    context: &CbPaymentContext,
    eligible: bool,
) -> &'static str {
    if eligible {
        return "eligible";
    }
    if !cb_card_context(context) {
        return "cb_card_context_required";
    }
    match service {
        CbServiceKind::SafeR if context.recurring => "safe_r_excludes_recurring",
        CbServiceKind::SafeR | CbServiceKind::FastR if !context.customer_initiated => {
            "customer_initiated_required"
        }
        CbServiceKind::UpdatR if !context.stored_credential => "stored_credential_required",
        _ => "not_eligible",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn french_cit_context() -> CbPaymentContext {
        CbPaymentContext {
            customer_initiated: true,
            recurring: false,
            stored_credential: false,
            card_country: Some("FR".to_string()),
            merchant_country: Some("FR".to_string()),
        }
    }

    #[test]
    fn cb_service_codes_are_provider_neutral() {
        assert_eq!(cb_service_codes(), ["safe_r", "updat_r", "fast_r"]);
    }

    #[test]
    fn safe_r_is_limited_to_customer_initiated_non_recurring_cb_payments() {
        let eligible = evaluate_cb_capability(CbServiceKind::SafeR, &french_cit_context());
        assert!(eligible.eligible);
        assert_eq!(eligible.provider, ProviderCode::Cb);
        assert_eq!(eligible.reason, "eligible");

        let recurring = CbPaymentContext {
            recurring: true,
            ..french_cit_context()
        };
        let rejected = evaluate_cb_capability(CbServiceKind::SafeR, &recurring);
        assert!(!rejected.eligible);
        assert_eq!(rejected.reason, "safe_r_excludes_recurring");
    }

    #[test]
    fn updat_r_requires_stored_credential_cb_context() {
        let rejected = evaluate_cb_capability(CbServiceKind::UpdatR, &french_cit_context());
        assert!(!rejected.eligible);
        assert_eq!(rejected.reason, "stored_credential_required");

        let stored = CbPaymentContext {
            stored_credential: true,
            ..french_cit_context()
        };
        assert!(evaluate_cb_capability(CbServiceKind::UpdatR, &stored).eligible);
    }

    #[test]
    fn fast_r_requires_customer_initiated_cb_ecommerce_context() {
        let merchant_initiated = CbPaymentContext {
            customer_initiated: false,
            ..french_cit_context()
        };
        let rejected = evaluate_cb_capability(CbServiceKind::FastR, &merchant_initiated);
        assert!(!rejected.eligible);
        assert_eq!(rejected.reason, "customer_initiated_required");
    }

    #[test]
    fn cb_integration_port_is_acquirer_or_pat_backed() {
        assert_eq!(
            validate_cb_integration_port(&CbIntegrationPort {
                acquirer_reference: None,
                pat_reference: None,
                enabled_services: vec![CbServiceKind::SafeR],
            }),
            Err("cb_acquirer_or_pat_required")
        );
        assert!(
            validate_cb_integration_port(&CbIntegrationPort {
                acquirer_reference: Some("acquirer-contract-eu-1".to_string()),
                pat_reference: None,
                enabled_services: vec![CbServiceKind::SafeR, CbServiceKind::UpdatR],
            })
            .is_ok()
        );
    }
}
