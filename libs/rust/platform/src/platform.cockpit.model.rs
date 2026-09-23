use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceId {
    Identity,
    Account,
    Billing,
    Email,
    TrustRisk,
}

impl ServiceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Identity => "identity-service",
            Self::Account => "account-service",
            Self::Billing => "billing-service",
            Self::Email => "email-worker",
            Self::TrustRisk => "trust-risk-service",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ServiceId;

    #[test]
    fn service_ids_are_stable_wire_names() {
        assert_eq!(ServiceId::Identity.as_str(), "identity-service");
        assert_eq!(ServiceId::TrustRisk.as_str(), "trust-risk-service");
        assert_eq!(
            serde_json::to_string(&ServiceId::Email).unwrap(),
            "\"email\""
        );
    }
}
