use crate::domains::authz::WorkspaceRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberListResponse {
    pub members: Vec<MemberView>,
    pub members_next_cursor: Option<String>,
    pub members_has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationListResponse {
    pub invitations: Vec<InvitationView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

pub struct ListMembersInput {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub role: Option<WorkspaceRole>,
    pub email_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberView {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationView {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteMemberResponse {
    pub invitation: InvitationView,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMemberResponse {
    pub member: MemberView,
    pub sessions_revoked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveMemberResponse {
    pub removed_user_id: Uuid,
    pub sessions_revoked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcceptInvitationResponse {
    pub workspace_id: Uuid,
    pub role: String,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct InviteMemberInput {
    pub email: String,
    pub role: WorkspaceRole,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberInput {
    pub role: WorkspaceRole,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationInput {
    pub token: String,
}

#[cfg(test)]
#[path = "drive.domains.members.types.tests.rs"]
mod tests;
