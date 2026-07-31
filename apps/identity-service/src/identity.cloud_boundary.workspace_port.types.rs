use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CloudWorkspaceSummary {
    pub workspace_id: Uuid,
    pub name: String,
    pub tenant_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub workspace_type: String,
    pub plan_code: String,
    pub data_region: Option<String>,
    pub jurisdiction: Option<String>,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
    pub required_acr: Option<String>,
    pub mfa_policy: Option<String>,
    pub owner_principal_id: Option<Uuid>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CloudWorkspaceMemberSummary {
    pub principal_id: Uuid,
    pub role: String,
    pub status: String,
    pub active: bool,
    pub joined_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
