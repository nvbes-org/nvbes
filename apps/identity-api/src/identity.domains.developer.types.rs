use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperContextResponse {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperOverviewResponse {
    pub tenant_id: Uuid,
    pub oauth_clients: i64,
    pub marketplace_pending: i64,
    pub high_risk_scopes: i64,
    pub failed_webhook_deliveries: i64,
    pub unhealthy_integrations: i64,
    pub active_sandboxes: i64,
}

#[derive(Debug, Serialize)]
pub struct DeveloperOAuthClientsResponse {
    pub oauth_clients: Vec<DeveloperOAuthClientSummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperOAuthClientSummary {
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub marketplace_status: Option<String>,
    pub consent_screen_configured: bool,
    pub redirect_uri_count: i64,
    pub allowed_scopes: Vec<String>,
    pub health_status: String,
}

#[derive(Debug, Serialize)]
pub struct DeveloperMarketplaceAppsResponse {
    pub apps: Vec<DeveloperMarketplaceAppSummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperMarketplaceAppSummary {
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub review_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperScopeRegistryResponse {
    pub scopes: Vec<DeveloperScopeRegistryEntry>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperScopeRegistryEntry {
    pub scope_key: String,
    pub display_name: String,
    pub description: String,
    pub risk: String,
    pub owner_team: String,
    pub lifecycle: String,
    pub allowed_audiences: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertDeveloperConsentScreenInput {
    pub product_name: String,
    pub logo_url: Option<String>,
    pub support_url: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub description: String,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperConsentScreenResponse {
    pub client_id: String,
    pub product_name: String,
    pub logo_url: Option<String>,
    pub support_url: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub description: String,
    pub configured: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperServiceAccountSummary {
    pub principal_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub role: String,
    pub status: String,
    pub workspace_id: Uuid,
    pub oauth_client_count: i64,
    pub last_rotated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperServiceAccountsResponse {
    pub service_accounts: Vec<DeveloperServiceAccountSummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperSecretVersionSummary {
    pub id: Uuid,
    pub client_id: String,
    pub status: String,
    pub secret_last4: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperSecretVersionsResponse {
    pub secret_versions: Vec<DeveloperSecretVersionSummary>,
}

#[derive(Debug, Deserialize)]
pub struct RotateDeveloperSecretInput {
    pub overlap_hours: i64,
}

#[derive(Debug, Serialize)]
pub struct RotateDeveloperSecretResponse {
    pub client_id: String,
    pub client_secret: String,
    pub active_version_id: Uuid,
    pub previous_version_id: Uuid,
    pub overlap_ends_at: DateTime<Utc>,
    pub rotated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperWebhookEndpointSummary {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub status: String,
    pub failed_delivery_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperWebhookEndpointsResponse {
    pub webhooks: Vec<DeveloperWebhookEndpointSummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperWebhookDeliverySummary {
    pub id: Uuid,
    pub endpoint_id: Uuid,
    pub event_id: Uuid,
    pub event_type: String,
    pub status: String,
    pub attempt_count: i32,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub replayed_from_delivery_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperWebhookDeliveriesResponse {
    pub deliveries: Vec<DeveloperWebhookDeliverySummary>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperConsoleLogEntry {
    pub id: Uuid,
    pub source: String,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperConsoleLogsResponse {
    pub logs: Vec<DeveloperConsoleLogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct DebugDeveloperTokenInput {
    pub access_token: String,
}

#[derive(Debug, Serialize)]
pub struct DebugDeveloperTokenResponse {
    pub active: bool,
    pub access_decision: String,
    pub claims: Option<DeveloperTokenClaimsView>,
    pub token_hash_prefix: String,
}

#[derive(Debug, Serialize)]
pub struct DeveloperTokenClaimsView {
    pub subject: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub client_id: Option<String>,
    pub scopes: Vec<String>,
    pub audience: String,
    pub issuer: String,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
    pub token_type: String,
    pub amr: Vec<String>,
    pub acr: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperSandboxTenantSummary {
    pub tenant_id: Uuid,
    pub sandbox_tenant_id: Uuid,
    pub sandbox_name: String,
    pub sandbox_slug: String,
    pub status: String,
    pub data_profile: String,
    pub reset_requested_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperSandboxResponse {
    pub sandbox: Option<DeveloperSandboxTenantSummary>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertDeveloperSandboxInput {
    pub data_profile: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct DeveloperHealthCheckSummary {
    pub id: Uuid,
    pub target_type: String,
    pub target_id: String,
    pub check_kind: String,
    pub status: String,
    pub summary: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperHealthChecksResponse {
    pub checks: Vec<DeveloperHealthCheckSummary>,
}

#[derive(Debug, Serialize)]
pub struct RunDeveloperHealthChecksResponse {
    pub checks: Vec<DeveloperHealthCheckSummary>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperHealthCheckSeed {
    pub target_type: String,
    pub target_id: String,
    pub check_kind: String,
    pub status: String,
    pub summary: String,
    pub metadata: Value,
}
