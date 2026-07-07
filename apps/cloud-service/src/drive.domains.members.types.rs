use crate::domains::authz::WorkspaceRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberListResponse {
    pub members: Vec<MemberView>,
    pub invitations: Vec<InvitationView>,
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
