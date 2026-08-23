use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct WorkspaceListResponse {
    pub workspaces: Vec<WorkspaceView>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceResponse {
    pub workspace: WorkspaceView,
}

#[derive(Debug, Serialize, Clone)]
pub struct WorkspaceView {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub role: String,
    pub plan_code: String,
    pub owner_principal_id: Uuid,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub policy: WorkspacePolicyView,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WorkspacePolicyView {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}

pub struct CreateWorkspaceInput {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub workspace_type: Option<String>,
}

pub struct UpdateWorkspaceInput {
    pub name: Option<String>,
    pub policy: Option<UpdateWorkspacePolicyInput>,
}

pub struct UpdateWorkspacePolicyInput {
    pub member_can_create_share_links: Option<bool>,
    pub require_admin_approval_for_member_share: Option<bool>,
    pub default_share_link_ttl_days: Option<i32>,
    pub max_share_link_ttl_days: Option<i32>,
}

pub struct NormalizedPolicy {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}
