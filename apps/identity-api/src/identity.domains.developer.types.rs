use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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
