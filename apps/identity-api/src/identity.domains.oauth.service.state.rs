use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AssuranceContext {
    pub acr: String,
    pub amr: Vec<String>,
    pub auth_time: i64,
    pub sufficient: bool,
}

#[derive(Debug, FromRow)]
pub struct AuthorizationCodeRecord {
    pub code: String,
    pub client_id: String,
    pub user_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub client_session_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub scope: String,
    pub redirect_uri: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub authorization_details: Vec<serde_json::Value>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug)]
pub struct SessionAssuranceState {
    pub created_at: DateTime<Utc>,
    pub step_up_verified_at: Option<DateTime<Utc>>,
    pub step_up_expires_at: Option<DateTime<Utc>>,
    pub acr: Option<String>,
    pub amr: Vec<String>,
}

#[derive(Debug)]
pub struct PolicyEvaluation {
    pub status: String,
    pub normalized_scope: Vec<String>,
}

#[derive(Debug)]
pub struct ConsentRequirementInput {
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub scope: Vec<String>,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub consent_action: Option<String>,
    pub policy_status: String,
}
