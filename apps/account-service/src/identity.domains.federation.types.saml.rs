use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct SamlSpConfigResponse {
    pub config: SamlSpConfigView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SamlSpConfigsResponse {
    pub configs: Vec<SamlSpConfigView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SamlAuthnRequestResponse {
    pub authn_request: String,
    pub redirect_url: String,
    pub request_id: String,
    pub relay_state: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct SamlAuthnRequestInput {
    pub sp_entity_id: String,
    pub acs_url: String,
    pub idp_sso_url: String,
    pub name_id_format: Option<String>,
    pub relay_state: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct CreateSamlSpConfigInput {
    pub entity_id: String,
    pub acs_url: String,
    pub slo_url: Option<String>,
    pub signing_cert_pem: Option<String>,
    pub signing_key_pem: Option<String>,
}

#[derive(Debug, ToSchema)]
pub struct UpdateSamlSpConfigInput {
    pub entity_id: Option<String>,
    pub acs_url: Option<String>,
    pub slo_url: Option<String>,
    pub signing_cert_pem: Option<String>,
    pub signing_key_pem: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SamlSpConfigView {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub entity_id: String,
    pub acs_url: String,
    pub slo_url: Option<String>,
    pub has_signing_cert: bool,
    pub has_signing_key: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SamlSpConfigRecord {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub entity_id: String,
    pub acs_url: String,
    pub slo_url: Option<String>,
    pub signing_cert_pem: Option<String>,
    pub signing_key_pem: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl SamlSpConfigRecord {
    pub fn into_view(self) -> SamlSpConfigView {
        SamlSpConfigView {
            id: self.id,
            tenant_id: self.tenant_id,
            entity_id: self.entity_id,
            acs_url: self.acs_url,
            slo_url: self.slo_url,
            has_signing_cert: self.signing_cert_pem.is_some(),
            has_signing_key: self.signing_key_pem.is_some(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
