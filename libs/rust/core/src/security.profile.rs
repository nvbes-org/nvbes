use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    #[default]
    Restricted,
}

impl DataClassification {
    pub fn is_customer_sensitive(self) -> bool {
        matches!(self, Self::Confidential | Self::Restricted)
    }

    pub fn allows_plaintext_observability(self) -> bool {
        matches!(self, Self::Public | Self::Internal)
    }

    pub fn requires_dedicated_controls(self) -> bool {
        matches!(self, Self::Restricted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecurityProfile {
    BrowserSession,
    OauthPublic,
    OauthConfidential,
    ServiceM2m,
    PartnerApi,
    RestrictedTenant,
}

impl SecurityProfile {
    pub fn requires_csrf(self) -> bool {
        matches!(self, Self::BrowserSession)
    }

    pub fn requires_sender_constrained_token(self) -> bool {
        matches!(
            self,
            Self::OauthConfidential | Self::ServiceM2m | Self::PartnerApi | Self::RestrictedTenant
        )
    }

    pub fn requires_mtls(self) -> bool {
        matches!(self, Self::ServiceM2m | Self::RestrictedTenant)
    }

    pub fn requires_http_message_signature(self) -> bool {
        matches!(self, Self::PartnerApi)
    }

    pub fn minimum_admin_aal(self) -> &'static str {
        match self {
            Self::RestrictedTenant => "aal3",
            Self::BrowserSession
            | Self::OauthPublic
            | Self::OauthConfidential
            | Self::ServiceM2m
            | Self::PartnerApi => "aal2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TenantCellKind {
    SharedDataPlane,
    DedicatedDataPlane,
    FullyDedicated,
}

impl TenantCellKind {
    pub fn allows_restricted_data(self) -> bool {
        matches!(self, Self::DedicatedDataPlane | Self::FullyDedicated)
    }

    pub fn requires_dedicated_kms_key(self) -> bool {
        matches!(self, Self::DedicatedDataPlane | Self::FullyDedicated)
    }
}

#[cfg(test)]
mod tests {
    use super::{DataClassification, SecurityProfile, TenantCellKind};

    #[test]
    fn restricted_is_the_default_classification() {
        assert_eq!(
            DataClassification::default(),
            DataClassification::Restricted
        );
    }

    #[test]
    fn restricted_profile_requires_high_assurance_controls() {
        let profile = SecurityProfile::RestrictedTenant;

        assert!(profile.requires_sender_constrained_token());
        assert!(profile.requires_mtls());
        assert_eq!(profile.minimum_admin_aal(), "aal3");
    }

    #[test]
    fn shared_cell_cannot_host_restricted_data() {
        assert!(!TenantCellKind::SharedDataPlane.allows_restricted_data());
        assert!(TenantCellKind::DedicatedDataPlane.allows_restricted_data());
    }
}
