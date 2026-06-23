use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<ApiKeyView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeApiKeyResponse {
    pub api_key: ApiKeyView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiIdentityResponse {
    pub api_key_id: Uuid,
    pub workspace_id: Uuid,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub rate_limit: RateLimitView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicWorkspacesResponse {
    pub workspaces: Vec<PublicWorkspaceView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_used_ip: Option<String>,
    pub http_signature_public_key: Option<String>,
    pub deprecated: bool,
    pub sunset_at: Option<DateTime<Utc>>,
    pub migration_target: ApiKeyMigrationTargetView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyMigrationTargetView {
    pub workspace_id: Uuid,
    pub identity_management_path: String,
    pub recommended_flow: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicWorkspaceView {
    pub id: Uuid,
    pub name: String,
    pub plan_code: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RateLimitView {
    pub requests_per_minute: usize,
    pub requests_per_day: usize,
}

pub struct PublicApiLogInput<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub status_code: i32,
    pub error_code: Option<&'a str>,
    pub scopes_used: &'a [&'a str],
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

pub struct PublicApiAuditEventInput<'a> {
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PublicApiContext {
    pub api_key_id: Option<Uuid>,
    pub workspace_id: Uuid,
    pub created_by: Option<Uuid>,
    pub created_by_principal_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub role: Option<String>,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub plan_code: String,
    pub request_id: String,
    pub m2m_client_id: Option<String>,
}

pub struct DeniedLogInput<'a> {
    pub workspace_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub request_id: &'a str,
    pub error_code: &'a str,
    pub network_block_reason: Option<&'a str>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub scopes_used: &'a [&'a str],
}

pub struct ApiRequestLogInsert<'a> {
    pub workspace_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub request_id: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub status_code: i32,
    pub error_code: Option<&'a str>,
    pub scopes_used: &'a [&'a str],
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}
