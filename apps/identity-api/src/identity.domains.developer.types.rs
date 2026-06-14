use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::rbac::{DeveloperPermission, DeveloperRole};

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperMeResponse {
    pub tenant_id: Uuid,
    pub roles: Vec<DeveloperRole>,
    pub permissions: Vec<DeveloperPermission>,
}

impl DeveloperMeResponse {
    pub fn new(
        tenant_id: Uuid,
        roles: Vec<DeveloperRole>,
        permissions: Vec<DeveloperPermission>,
    ) -> Self {
        Self {
            tenant_id,
            roles,
            permissions,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperAppView {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub client_type: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperAppsResponse {
    pub apps: Vec<DeveloperAppView>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeveloperAppRequest {
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    #[serde(default)]
    pub allowed_audiences: Vec<String>,
    #[serde(default)]
    pub allowed_resources: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDeveloperAppResponse {
    pub app: DeveloperAppView,
    pub client_secret: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDeveloperRedirectsRequest {
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum DeveloperWebhookEventType {
    #[serde(rename = "user.created")]
    UserCreated,
    #[serde(rename = "login.failed")]
    LoginFailed,
    #[serde(rename = "session.revoked")]
    SessionRevoked,
    #[serde(rename = "client.created")]
    ClientCreated,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWebhookEndpointView {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub status: String,
    pub events: Vec<DeveloperWebhookEventType>,
    pub signing_secret_last4: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeveloperWebhookEndpointRequest {
    pub name: String,
    pub url: String,
    pub events: Vec<DeveloperWebhookEventType>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDeveloperWebhookEndpointResponse {
    pub endpoint: DeveloperWebhookEndpointView,
    pub signing_secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWebhooksResponse {
    pub endpoints: Vec<DeveloperWebhookEndpointView>,
}

impl DeveloperWebhookEventType {
    pub fn as_event_type(self) -> &'static str {
        match self {
            DeveloperWebhookEventType::UserCreated => "user.created",
            DeveloperWebhookEventType::LoginFailed => "login.failed",
            DeveloperWebhookEventType::SessionRevoked => "session.revoked",
            DeveloperWebhookEventType::ClientCreated => "client.created",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InspectDeveloperTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InspectDeveloperTokenResponse {
    pub active: bool,
    pub subject: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthPlaygroundExchangeRequest {
    pub client_id: String,
    pub code: String,
    pub redirect_uri: String,
    pub code_verifier: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthPlaygroundExchangeResponse {
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperLogEntry {
    pub id: String,
    pub event_type: String,
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperLogsResponse {
    pub logs: Vec<DeveloperLogEntry>,
}
