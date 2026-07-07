use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::grpc::pb::nvbes::cloud::v1 as cloud;

#[derive(Debug, FromRow)]
pub struct WorkspaceRow {
    pub workspace_id: Uuid,
    pub name: String,
    pub data_region: String,
    pub jurisdiction: String,
    pub owner_principal_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_type: String,
    pub plan_code: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
    pub required_acr: String,
    pub mfa_policy: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct WorkspaceMemberRow {
    pub workspace_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub status: String,
    pub source: String,
    pub joined_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct WorkspaceInvitationRow {
    pub invitation_id: Uuid,
    pub workspace_id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl WorkspaceRow {
    pub fn into_proto(self) -> cloud::Workspace {
        cloud::Workspace {
            workspace_id: self.workspace_id.to_string(),
            name: self.name,
            region_id: self.data_region,
            data_residency: self.jurisdiction,
            status: cloud::WorkspaceStatus::Active as i32,
            tenant_id: self.tenant_id.map(|id| id.to_string()).unwrap_or_default(),
            organization_id: self
                .organization_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            workspace_type: self.workspace_type,
            plan_code: self.plan_code,
            policy: Some(cloud::WorkspacePolicy {
                member_can_create_share_links: self.member_can_create_share_links,
                require_admin_approval_for_member_share: self
                    .require_admin_approval_for_member_share,
                default_share_link_ttl_days: self.default_share_link_ttl_days,
                max_share_link_ttl_days: self.max_share_link_ttl_days,
                required_acr: self.required_acr,
                mfa_policy: self.mfa_policy,
            }),
            owner_principal_id: self.owner_principal_id.to_string(),
            trial_ends_at: self
                .trial_ends_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

impl WorkspaceMemberRow {
    pub fn into_proto(self) -> cloud::WorkspaceMember {
        cloud::WorkspaceMember {
            workspace_id: self.workspace_id.to_string(),
            principal_id: self.principal_id.to_string(),
            role: self.role,
            status: member_status(&self.status) as i32,
            joined_at: self.joined_at.to_rfc3339(),
            source: self.source,
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

impl WorkspaceInvitationRow {
    pub fn into_proto(self) -> cloud::WorkspaceInvitation {
        cloud::WorkspaceInvitation {
            invitation_id: self.invitation_id.to_string(),
            workspace_id: self.workspace_id.to_string(),
            email: self.email,
            role: self.role,
            status: self.status,
            expires_at: self.expires_at.to_rfc3339(),
            accepted_at: self
                .accepted_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
            revoked_at: self
                .revoked_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
            created_at: self.created_at.to_rfc3339(),
        }
    }
}

fn member_status(value: &str) -> cloud::WorkspaceMemberStatus {
    match value {
        "active" => cloud::WorkspaceMemberStatus::Active,
        "suspended" => cloud::WorkspaceMemberStatus::Suspended,
        "removed" => cloud::WorkspaceMemberStatus::Removed,
        _ => cloud::WorkspaceMemberStatus::Unspecified,
    }
}
