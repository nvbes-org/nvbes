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
    fn data_classification_flags_are_exclusive() {
        assert!(DataClassification::Confidential.is_customer_sensitive());
        assert!(DataClassification::Restricted.is_customer_sensitive());
        assert!(!DataClassification::Public.is_customer_sensitive());
        assert!(!DataClassification::Internal.is_customer_sensitive());

        assert!(DataClassification::Public.allows_plaintext_observability());
        assert!(DataClassification::Internal.allows_plaintext_observability());
        assert!(!DataClassification::Confidential.allows_plaintext_observability());
        assert!(!DataClassification::Restricted.allows_plaintext_observability());

        assert!(DataClassification::Restricted.requires_dedicated_controls());
        assert!(!DataClassification::Confidential.requires_dedicated_controls());
        assert!(!DataClassification::Public.requires_dedicated_controls());
    }

    #[test]
    fn security_profile_controls_match_each_variant() {
        assert!(SecurityProfile::BrowserSession.requires_csrf());
        assert!(!SecurityProfile::OauthPublic.requires_csrf());
        assert!(!SecurityProfile::ServiceM2m.requires_csrf());

        assert!(!SecurityProfile::BrowserSession.requires_sender_constrained_token());
        assert!(!SecurityProfile::OauthPublic.requires_sender_constrained_token());
        assert!(SecurityProfile::OauthConfidential.requires_sender_constrained_token());
        assert!(SecurityProfile::ServiceM2m.requires_sender_constrained_token());
        assert!(SecurityProfile::PartnerApi.requires_sender_constrained_token());
        assert!(SecurityProfile::RestrictedTenant.requires_sender_constrained_token());

        assert!(!SecurityProfile::BrowserSession.requires_mtls());
        assert!(!SecurityProfile::PartnerApi.requires_mtls());
        assert!(SecurityProfile::ServiceM2m.requires_mtls());
        assert!(SecurityProfile::RestrictedTenant.requires_mtls());

        assert!(SecurityProfile::PartnerApi.requires_http_message_signature());
        assert!(!SecurityProfile::ServiceM2m.requires_http_message_signature());
        assert!(!SecurityProfile::BrowserSession.requires_http_message_signature());

        assert_eq!(
            SecurityProfile::RestrictedTenant.minimum_admin_aal(),
            "aal3"
        );
        assert_eq!(SecurityProfile::BrowserSession.minimum_admin_aal(), "aal2");
        assert_eq!(SecurityProfile::ServiceM2m.minimum_admin_aal(), "aal2");
        assert_eq!(SecurityProfile::PartnerApi.minimum_admin_aal(), "aal2");
    }

    #[test]
    fn tenant_cell_kind_gates_restricted_data_and_kms() {
        assert!(!TenantCellKind::SharedDataPlane.allows_restricted_data());
        assert!(TenantCellKind::DedicatedDataPlane.allows_restricted_data());
        assert!(TenantCellKind::FullyDedicated.allows_restricted_data());

        assert!(!TenantCellKind::SharedDataPlane.requires_dedicated_kms_key());
        assert!(TenantCellKind::DedicatedDataPlane.requires_dedicated_kms_key());
        assert!(TenantCellKind::FullyDedicated.requires_dedicated_kms_key());
    }
}
