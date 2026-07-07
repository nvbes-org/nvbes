use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceAccountsResult {
    pub service_accounts: Vec<ServiceAccountView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceAccountView {
    pub principal_id: Uuid,
    pub tenant_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub oauth_clients: Vec<ServiceAccountClientView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceAccountClientView {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub client_assertion_required: bool,
    pub client_assertion_public_key_configured: bool,
    pub allowed_scopes: Vec<String>,
    pub allowed_audiences: Vec<String>,
    pub allowed_resources: Vec<String>,
    pub required_acr: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateServiceAccountInput {
    pub name: String,
    pub description: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateServiceAccountInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateServiceAccountOAuthClientInput {
    pub name: String,
    pub allowed_scopes: Vec<String>,
    #[serde(default)]
    pub allowed_audiences: Vec<String>,
    #[serde(default)]
    pub allowed_resources: Vec<String>,
    pub required_acr: Option<String>,
    pub client_assertion_public_key_jwk: Option<serde_json::Value>,
    pub client_assertion_required: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateServiceAccountOAuthClientResult {
    pub client: ServiceAccountClientView,
    pub client_secret: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttachOAuthClientInput {
    pub client_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateOAuthClientSecretResult {
    pub client_id: String,
    pub client_secret: String,
    pub rotated_at: DateTime<Utc>,
}
