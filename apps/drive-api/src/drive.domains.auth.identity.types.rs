use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IdentityIntrospectionResponse {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub principal_type: Option<String>,
    pub token_type: Option<String>,
    pub sub: Option<String>,
    pub role: Option<String>,
    pub tenant_id: Option<uuid::Uuid>,
    pub organization_id: Option<uuid::Uuid>,
    pub workspace_id: Option<uuid::Uuid>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    #[serde(alias = "display_name")]
    pub name: Option<String>,
    pub acr: Option<String>,
    #[serde(default)]
    pub amr: Vec<String>,
    pub auth_time: Option<i64>,
    pub jti: Option<String>,
    pub sid: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub nbf: Option<i64>,
    #[serde(default)]
    pub act: Option<serde_json::Value>,
    pub actor_principal_type: Option<String>,
    pub actor_role: Option<String>,
    pub actor_workspace_id: Option<uuid::Uuid>,
    pub actor_organization_id: Option<uuid::Uuid>,
    pub actor_tenant_id: Option<uuid::Uuid>,
    pub network_valid: Option<bool>,
}
