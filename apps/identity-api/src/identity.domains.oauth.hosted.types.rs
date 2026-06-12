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
    pub request_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostedClientDisplay {
    pub client_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum HostedLoginDecision {
    LoginRequired {
        login_url: String,
    },
    ConsentRequired {
        state_id: String,
        client: HostedClientDisplay,
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
