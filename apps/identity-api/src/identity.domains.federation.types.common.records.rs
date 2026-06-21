use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::{
    FederatedIdentityProviderView, LinkedIdentityView, ScimProvisioningConnectorView,
    TenantDomainView,
};

#[derive(Debug, FromRow)]
pub struct TenantDomainRecord {
    pub id: Uuid,
    pub domain: String,
    pub sso_required: bool,
    pub sso_provider_id: Option<Uuid>,
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
            sso_required: self.sso_required,
            sso_provider_id: self.sso_provider_id,
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

impl FederatedIdentityProviderRecord {
    pub fn into_view(self) -> FederatedIdentityProviderView {
        FederatedIdentityProviderView {
            id: self.id,
            provider_type: self.provider_type,
            provider_family: self.provider_family,
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
