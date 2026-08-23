use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedHostedAuthorizationState {
    pub state_id: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub nonce: Option<String>,
    pub audience: Option<String>,
    #[serde(default)]
    pub resource_indicators: Vec<String>,
    #[serde(default)]
    pub authorization_details: crate::domains::oauth::rar::AuthorizationDetails,
    pub request_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub dpop_jkt: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostedClientDisplay {
    pub client_id: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub description: Option<String>,
    pub support_url: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub brand_color: Option<String>,
    pub custom_css: Option<String>,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostedLoginDecision {
    LoginRequired {
        login_url: String,
        state_id: String,
    },
    ConsentRequired {
        state_id: String,
        client: Box<HostedClientDisplay>,
        scope: String,
    },
    Redirect {
        redirect_url: String,
    },
    ErrorPage {
        code: String,
        message: String,
    },
}

#[cfg(test)]
#[path = "identity.domains.oauth.hosted.types.tests.rs"]
mod tests;
