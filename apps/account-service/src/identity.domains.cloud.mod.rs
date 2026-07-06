#[path = "identity.domains.cloud.workspace_port.rs"]
pub mod workspace_port;
#[path = "identity.domains.cloud.workspace_projection.rs"]
pub mod workspace_projection;

pub use workspace_port::{
    CloudWorkspaceMemberSummary, CloudWorkspaceSummary, accept_workspace_invitation_tx,
    create_workspace_invitation_tx, create_workspace_tx, get_workspace,
    get_workspace_invitation_by_token_hash, list_tenant_workspaces, list_workspace_invitations,
    list_workspace_members, list_workspaces, update_workspace_membership_role_tx,
    update_workspace_membership_status_tx, update_workspace_settings_tx,
    upsert_workspace_membership_tx,
};
