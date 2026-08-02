use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceView {
    pub id: Uuid,
    pub owner_principal_id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: String,
    pub role: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SessionView {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
    pub geo_country_code: Option<String>,
    pub user_agent: Option<String>,
    pub client: Option<crate::domains::auth::user_agent::UserAgentInfo>,
    pub device_id: Option<Uuid>,
    pub device_trust_level: Option<String>,
    pub device_trust_score: Option<i16>,
    pub risk_score: Option<f64>,
    pub risk_decision: Option<String>,
    pub risk_confirmed_at: Option<DateTime<Utc>>,
    pub current: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionsResult {
    pub sessions: Vec<SessionView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwitchWorkspaceResult {
    pub workspace: WorkspaceView,
    pub session: SessionView,
    pub stepped_up: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub browser_session_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{SessionView, SessionsResult};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn sessions_result_serializes_pagination_metadata() {
        let payload = serde_json::to_value(SessionsResult {
            sessions: vec![SessionView {
                id: Uuid::from_u128(1),
                tenant_id: None,
                organization_id: None,
                workspace_id: None,
                workspace_region: None,
                created_at: Utc::now(),
                last_seen_at: Utc::now(),
                expires_at: Utc::now(),
                revoked_at: None,
                ip: None,
                geo_country_code: None,
                user_agent: None,
                client: None,
                device_id: None,
                device_trust_level: None,
                device_trust_score: None,
                risk_score: None,
                risk_decision: None,
                risk_confirmed_at: None,
                current: true,
            }],
            next_cursor: Some("opaque-cursor".to_string()),
            has_more: true,
        })
        .expect("serializes");

        assert_eq!(payload["next_cursor"], "opaque-cursor");
        assert_eq!(payload["has_more"], true);
    }
}
