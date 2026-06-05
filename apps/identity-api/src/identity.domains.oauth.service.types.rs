#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::oauth::rar::AuthorizationDetails;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceAuthorizationInput {
    pub client_id: String,
    pub scope: String,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceAuthorizationView {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: i64,
    pub interval: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExchangeDeviceCodeInput {
    pub client_id: String,
    pub device_code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceVerificationInput {
    pub user_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceVerificationView {
    pub client_name: String,
    pub scope: Vec<String>,
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceApprovalInput {
    pub user_code: String,
    pub workspace_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub consent_action: Option<String>,
}

#[derive(Debug)]
pub struct CreateAuthorizationCodeInput {
    pub client_id: String,
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub scope: String,
    pub redirect_uri: String,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub authorization_details: AuthorizationDetails,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub consent_action: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthorizationCodeView {
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ExchangeCodeInput {
    pub code: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_assertion_verified: bool,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClientAssertionAuthentication {
    pub assertion_type: String,
    pub assertion: String,
}

#[derive(Debug, Clone)]
pub struct ClientAuthentication {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_assertion: Option<ClientAssertionAuthentication>,
    pub client_assertion_verified: bool,
}

impl ClientAuthentication {
    pub fn uses_private_key_jwt(&self) -> bool {
        self.client_assertion.is_some()
    }
}

#[derive(Debug)]
pub struct AuthCodeRecord {
    pub user_id: Uuid,
    pub client_session_id: Option<Uuid>,
    pub scope: String,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub authorization_details: AuthorizationDetails,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub client_id: String,
    pub client_uuid: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenView {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    #[schema(value_type = Vec<Object>)]
    pub authorization_details: AuthorizationDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_token_type: Option<String>,
}

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

#[derive(Debug)]
pub struct TokenExchangeInput {
    pub subject_token: String,
    pub subject_token_type: String,
    pub actor_token: Option<String>,
    pub actor_token_type: Option<String>,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_assertion_verified: bool,
    pub scope: Option<String>,
    pub audience: Option<String>,
    pub resource: Option<String>,
    pub requested_token_type: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParResponse {
    pub request_uri: String,
    pub expires_in: i64,
}
