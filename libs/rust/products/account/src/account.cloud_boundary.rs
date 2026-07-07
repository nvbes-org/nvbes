use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePolicyCommand {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
    pub required_acr: Option<String>,
    pub mfa_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkspaceCommand {
    pub workspace_id: Uuid,
    pub tenant_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub owner_principal_id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub plan_code: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub data_region: String,
    pub jurisdiction: String,
    pub owner_role: String,
    pub membership_source: String,
    pub created_at: DateTime<Utc>,
    pub policy: WorkspacePolicyCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateWorkspaceSettingsCommand {
    pub actor_principal_id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub policy: WorkspacePolicyCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertWorkspaceMembershipCommand {
    pub actor_principal_id: Uuid,
    pub workspace_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub status: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateWorkspaceMembershipRoleCommand {
    pub actor_principal_id: Uuid,
    pub workspace_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateWorkspaceMembershipStatusCommand {
    pub actor_principal_id: Uuid,
    pub workspace_id: Uuid,
    pub principal_id: Uuid,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkspaceInvitationCommand {
    pub workspace_id: Uuid,
    pub email: String,
    pub role: String,
    pub invited_by: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptWorkspaceInvitationCommand {
    pub invitation_id: Uuid,
    pub workspace_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub accepted_at: DateTime<Utc>,
}
