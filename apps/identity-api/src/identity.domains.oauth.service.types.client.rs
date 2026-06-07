use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthClientView {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub tenant_id: Option<Uuid>,
    pub owner_scope_type: String,
    pub owner_scope_id: Uuid,
    pub client_type: String,
    pub client_assertion_required: bool,
    pub client_assertion_public_key_configured: bool,
    pub service_account_principal_id: Option<Uuid>,
    pub service_account_workspace_id: Option<Uuid>,
    pub service_account_role: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthClientsResult {
    pub clients: Vec<OAuthClientView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthClientPolicyView {
    pub id: Uuid,
    pub client_id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub allowed_scopes: Vec<String>,
    pub allowed_audiences: Vec<String>,
    pub allowed_resources: Vec<String>,
    pub required_acr: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthClientPoliciesResult {
    pub policies: Vec<OAuthClientPolicyView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateOAuthClientResult {
    pub client: OAuthClientView,
    pub client_secret: String,
    pub policy: OAuthClientPolicyView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteOAuthClientPolicyResult {
    pub success: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeOAuthClientResult {
    pub client_id: String,
    pub tokens_revoked: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOAuthClientInput {
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    #[serde(default)]
    pub allowed_audiences: Vec<String>,
    #[serde(default)]
    pub allowed_resources: Vec<String>,
    pub required_acr: Option<String>,
    pub client_type: Option<String>,
    pub owner_scope_type: Option<String>,
    pub owner_scope_id: Option<Uuid>,
    pub client_assertion_public_key_jwk: Option<serde_json::Value>,
    pub client_assertion_required: Option<bool>,
    pub service_account_name: Option<String>,
    pub service_account_description: Option<String>,
    pub service_account_principal_id: Option<Uuid>,
    pub service_account_role: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOAuthClientPolicyInput {
    pub scope_type: Option<String>,
    pub scope_id: Option<Uuid>,
    pub allowed_scopes: Vec<String>,
    #[serde(default)]
    pub allowed_audiences: Vec<String>,
    #[serde(default)]
    pub allowed_resources: Vec<String>,
    pub required_acr: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOAuthClientPolicyInput {
    pub allowed_scopes: Option<Vec<String>>,
    pub allowed_audiences: Option<Vec<String>>,
    pub allowed_resources: Option<Vec<String>>,
    pub required_acr: Option<String>,
    pub status: Option<String>,
}
