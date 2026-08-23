use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::rbac::{DeveloperPermission, DeveloperRole};

#[path = "developer.http.types.sandbox.rs"]
mod sandbox;
#[path = "developer.http.types.service_accounts.rs"]
mod service_accounts;
#[path = "developer.http.types.tokens.rs"]
mod tokens;
#[path = "developer.http.types.webhooks.rs"]
mod webhooks;

pub use sandbox::{
    DeveloperHealthCheckSeed, DeveloperHealthCheckSummary, DeveloperHealthChecksResponse,
    DeveloperSandboxResponse, DeveloperSandboxTenantSummary, RunDeveloperHealthChecksResponse,
    UpsertDeveloperSandboxInput,
};
pub use service_accounts::{
    DeveloperSecretVersionSummary, DeveloperSecretVersionsResponse, DeveloperServiceAccountSummary,
    DeveloperServiceAccountsResponse, RotateDeveloperSecretInput, RotateDeveloperSecretResponse,
};
pub use tokens::{DebugDeveloperTokenInput, DebugDeveloperTokenResponse, DeveloperTokenClaimsView};
pub use webhooks::{
    CreateDeveloperWebhookEndpointRequest, CreateDeveloperWebhookEndpointResponse,
    DeveloperWebhookDeliveriesResponse, DeveloperWebhookDeliverySummary,
    DeveloperWebhookEndpointSummary, DeveloperWebhookEndpointView,
    DeveloperWebhookEndpointsResponse, DeveloperWebhookEventType, DeveloperWebhooksResponse,
};

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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertDeveloperConsentScreenInput {
    pub product_name: String,
    pub logo_url: Option<String>,
    pub support_url: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub description: String,
    pub brand_color: Option<String>,
    pub custom_css: Option<String>,
    pub help_text: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewMarketplaceAppInput {
    pub status: String,
    pub review_reason: Option<String>,
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
    pub brand_color: Option<String>,
    pub custom_css: Option<String>,
    pub help_text: Option<String>,
    pub configured: bool,
    pub updated_at: Option<DateTime<Utc>>,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateScopeInput {
    pub scope_key: String,
    pub display_name: String,
    pub description: String,
    pub risk: String,
    pub owner_team: String,
    pub lifecycle: Option<String>,
    pub allowed_audiences: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateScopeInput {
    pub display_name: String,
    pub description: String,
    pub risk: String,
    pub owner_team: String,
    pub lifecycle: String,
    pub allowed_audiences: Vec<String>,
}
