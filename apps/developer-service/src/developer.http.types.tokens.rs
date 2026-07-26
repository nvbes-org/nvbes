use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Token material accepted only for diagnostics and never echoed.
#[derive(Debug, Deserialize, ToSchema)]
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
