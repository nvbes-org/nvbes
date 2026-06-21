use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[path = "identity.domains.federation.types.common.records.rs"]
mod records;

pub use records::{
    FederatedIdentityProviderRecord, LinkedIdentityRecord, PrincipalRecord,
    ScimProvisioningConnectorRecord, TenantDomainRecord,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantDomainsResponse {
    pub domains: Vec<TenantDomainView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantDomainResponse {
    pub domain: TenantDomainView,
    pub verification_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FederatedIdentityProvidersResponse {
    pub providers: Vec<FederatedIdentityProviderView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FederatedIdentityProviderResponse {
    pub provider: FederatedIdentityProviderView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScimProvisioningConnectorsResponse {
    pub connectors: Vec<ScimProvisioningConnectorView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScimProvisioningConnectorResponse {
    pub connector: ScimProvisioningConnectorView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkedIdentitiesResponse {
    pub identities: Vec<LinkedIdentityView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkedIdentityResponse {
    pub identity: LinkedIdentityView,
}

#[derive(Debug, ToSchema)]
pub struct CreateTenantDomainInput {
    pub domain: String,
    pub sso_required: Option<bool>,
    pub sso_provider_id: Option<Uuid>,
}

#[derive(Debug, ToSchema)]
pub struct UpdateTenantDomainInput {
    pub sso_required: Option<bool>,
    pub sso_provider_id: Option<Uuid>,
}

#[derive(Debug, ToSchema)]
pub struct VerifyTenantDomainInput {
    pub token: String,
}

#[derive(Debug, ToSchema)]
pub struct CreateFederatedIdentityProviderInput {
    pub provider_type: String,
    pub provider_family: Option<String>,
    pub name: String,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub metadata_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct UpdateFederatedIdentityProviderInput {
    pub provider_type: Option<String>,
    pub provider_family: Option<String>,
    pub name: Option<String>,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub metadata_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct CreateScimProvisioningConnectorInput {
    pub provider: String,
    pub base_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct UpdateScimProvisioningConnectorInput {
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct CreateLinkedIdentityInput {
    pub principal_id: Uuid,
    pub provider_type: String,
    pub provider_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OidcDiscoveryResponse {
    pub discovery_url: String,
    pub issuer: String,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub userinfo_endpoint: Option<String>,
    pub jwks_uri: Option<String>,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub claims_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SamlMetadataResponse {
    pub entity_id: String,
    pub sso_url: Option<String>,
    pub signing_certificates: Vec<String>,
    pub supports_signed_assertions: bool,
    pub supports_signed_responses: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantDomainView {
    pub id: Uuid,
    pub domain: String,
    pub sso_required: bool,
    pub sso_provider_id: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_requested_at: Option<DateTime<Utc>>,
    pub verification_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FederatedIdentityProviderView {
    pub id: Uuid,
    pub provider_type: String,
    pub provider_family: String,
    pub name: String,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub metadata_url: Option<String>,
    pub status: String,
    pub sp_entity_id: Option<String>,
    pub attribute_mapping: serde_json::Value,
    pub encryption_cert_pem: Option<String>,
    pub require_signed_assertions: bool,
    pub require_signed_responses: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScimProvisioningConnectorView {
    pub id: Uuid,
    pub provider: String,
    pub status: String,
    pub base_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkedIdentityView {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub provider_type: String,
    pub provider_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}
