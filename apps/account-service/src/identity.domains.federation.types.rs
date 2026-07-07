#[path = "identity.domains.federation.types.common.rs"]
mod common;
#[path = "identity.domains.federation.types.provisioning.rs"]
mod provisioning;
#[path = "identity.domains.federation.types.saml.rs"]
mod saml;

pub use common::{
    CreateFederatedIdentityProviderInput, CreateLinkedIdentityInput,
    CreateScimProvisioningConnectorInput, CreateTenantDomainInput, FederatedIdentityProviderRecord,
    FederatedIdentityProviderResponse, FederatedIdentityProviderView,
    FederatedIdentityProvidersResponse, LinkedIdentitiesResponse, LinkedIdentityRecord,
    LinkedIdentityResponse, LinkedIdentityView, OidcDiscoveryResponse, PrincipalRecord,
    SamlMetadataResponse, ScimProvisioningConnectorRecord, ScimProvisioningConnectorResponse,
    ScimProvisioningConnectorView, ScimProvisioningConnectorsResponse, TenantDomainRecord,
    TenantDomainResponse, TenantDomainView, TenantDomainsResponse,
    UpdateFederatedIdentityProviderInput, UpdateScimProvisioningConnectorInput,
    UpdateTenantDomainInput, VerifyTenantDomainInput,
};
pub use provisioning::{
    InboundFederationInput, InboundFederationResponse, JitProvisioningInput,
    JitProvisioningResponse,
};
pub use saml::{
    CreateSamlSpConfigInput, SamlAuthnRequestInput, SamlAuthnRequestResponse, SamlSpConfigRecord,
    SamlSpConfigResponse, SamlSpConfigView, SamlSpConfigsResponse, UpdateSamlSpConfigInput,
};
