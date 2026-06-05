use uuid::Uuid;

use nvbes_core::authz::WorkspaceRole;

#[derive(Debug, Clone)]
pub struct WorkspaceAccess<Auth> {
    pub auth: Auth,
    pub workspace_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub role: WorkspaceRole,
    pub policy: WorkspacePolicy,
}

#[derive(Debug, Clone)]
pub struct WorkspacePolicy {
    pub member_can_create_share_links: bool,
}

impl<Auth> WorkspaceAccess<Auth> {
    pub fn new(
        auth: Auth,
        workspace_id: Uuid,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        role: WorkspaceRole,
        policy: WorkspacePolicy,
    ) -> Self {
        Self {
            auth,
            workspace_id,
            tenant_id,
            organization_id,
            role,
            policy,
        }
    }
}

impl WorkspacePolicy {
    pub fn member_share_links_enabled(member_can_create_share_links: bool) -> Self {
        Self {
            member_can_create_share_links,
        }
    }
}

pub fn role_as_db(role: WorkspaceRole) -> &'static str {
    match role {
        WorkspaceRole::Owner => "owner",
        WorkspaceRole::Admin => "admin",
        WorkspaceRole::Member => "member",
        WorkspaceRole::Viewer => "viewer",
    }
}

pub fn role_as_str(role: WorkspaceRole) -> &'static str {
    role_as_db(role)
}
