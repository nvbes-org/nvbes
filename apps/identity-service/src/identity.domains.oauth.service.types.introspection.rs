use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::oauth::rar::AuthorizationDetails;

#[derive(Debug, Serialize, ToSchema)]
pub struct IntrospectionResponse {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    #[schema(value_type = Vec<Object>)]
    pub authorization_details: AuthorizationDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acr: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub amr: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_principal_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_workspace_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_tenant_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_valid: Option<bool>,
}

impl IntrospectionResponse {
    pub fn inactive() -> Self {
        Self {
            active: false,
            scope: None,
            authorization_details: Vec::new(),
            client_id: None,
            principal_type: None,
            token_type: None,
            audience: None,
            sub: None,
            role: None,
            tenant_id: None,
            organization_id: None,
            workspace_id: None,
            username: None,
            email: None,
            email_verified: None,
            display_name: None,
            acr: None,
            amr: Vec::new(),
            auth_time: None,
            jti: None,
            sid: None,
            exp: None,
            iat: None,
            nbf: None,
            cnf: None,
            act: None,
            actor_principal_type: None,
            actor_role: None,
            actor_workspace_id: None,
            actor_organization_id: None,
            actor_tenant_id: None,
            network_valid: None,
        }
    }
}
