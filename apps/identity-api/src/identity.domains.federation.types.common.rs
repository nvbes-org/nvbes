use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

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
}

#[derive(Debug, ToSchema)]
pub struct VerifyTenantDomainInput {
    pub token: String,
}

#[derive(Debug, ToSchema)]
pub struct CreateFederatedIdentityProviderInput {
    pub provider_type: String,
    pub name: String,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub metadata_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct UpdateFederatedIdentityProviderInput {
    pub provider_type: Option<String>,
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
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_requested_at: Option<DateTime<Utc>>,
    pub verification_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FederatedIdentityProviderView {
    pub id: Uuid,
    pub provider_type: String,
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

#[derive(Debug, FromRow)]
pub struct TenantDomainRecord {
    pub id: Uuid,
    pub domain: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_requested_at: Option<DateTime<Utc>>,
    pub verification_expires_at: Option<DateTime<Utc>>,
    pub verification_token_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TenantDomainRecord {
    pub fn into_view(self) -> TenantDomainView {
        TenantDomainView {
            id: self.id,
            domain: self.domain,
            verified_at: self.verified_at,
            verification_requested_at: self.verification_requested_at,
            verification_expires_at: self.verification_expires_at,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct FederatedIdentityProviderRecord {
    pub id: Uuid,
    pub provider_type: String,
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

impl FederatedIdentityProviderRecord {
    pub fn into_view(self) -> FederatedIdentityProviderView {
        FederatedIdentityProviderView {
            id: self.id,
            provider_type: self.provider_type,
            name: self.name,
            client_id: self.client_id,
            issuer: self.issuer,
            metadata_url: self.metadata_url,
            status: self.status,
            sp_entity_id: self.sp_entity_id,
            attribute_mapping: self.attribute_mapping,
            encryption_cert_pem: self.encryption_cert_pem,
            require_signed_assertions: self.require_signed_assertions,
            require_signed_responses: self.require_signed_responses,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct ScimProvisioningConnectorRecord {
    pub id: Uuid,
    pub provider: String,
    pub status: String,
    pub base_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ScimProvisioningConnectorRecord {
    pub fn into_view(self) -> ScimProvisioningConnectorView {
        ScimProvisioningConnectorView {
            id: self.id,
            provider: self.provider,
            status: self.status,
            base_url: self.base_url,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct LinkedIdentityRecord {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub provider_type: String,
    pub provider_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl LinkedIdentityRecord {
    pub fn into_view(self) -> LinkedIdentityView {
        LinkedIdentityView {
            id: self.id,
            principal_id: self.principal_id,
            provider_type: self.provider_type,
            provider_id: self.provider_id,
            subject: self.subject,
            email: self.email,
            email_verified: self.email_verified,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PrincipalRecord {
    pub id: Uuid,
}
