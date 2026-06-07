use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[path = "identity.domains.auth.login_challenges.keys.rs"]
mod keys;
#[path = "identity.domains.auth.login_challenges.lifecycle.rs"]
mod lifecycle;
#[path = "identity.domains.auth.login_challenges.state.rs"]
mod state;
#[path = "identity.domains.auth.login_challenges.storage.rs"]
mod storage;
#[path = "identity.domains.auth.login_challenges.tests.rs"]
mod tests;

pub(crate) const MAX_FAILED_ATTEMPTS: i32 = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateLoginChallengeInput {
    pub auth_state_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub purpose: &'static str,
    pub required_level: &'static str,
    pub allowed_factor_types: Vec<&'static str>,
    pub factor_id: Option<Uuid>,
    pub metadata: Value,
    pub ttl_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLoginChallenge {
    pub id: Uuid,
    pub auth_state_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub purpose: String,
    pub required_level: String,
    pub allowed_factor_types: Vec<String>,
    pub factor_id: Option<Uuid>,
    pub metadata: Value,
    pub failed_attempts: i32,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct LoginChallenge {
    pub id: Uuid,
    pub auth_state_id: Uuid,
    pub principal_id: Option<Uuid>,
    pub metadata: Value,
    pub allowed_factor_types: Vec<String>,
    pub failed_attempts: i32,
}

pub use lifecycle::{prune_expired_challenges, replace_challenge};
pub use state::{consume_challenge, fetch_active_challenge, get_challenge, record_failed_attempt};
