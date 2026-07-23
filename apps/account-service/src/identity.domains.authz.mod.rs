#[path = "identity.domains.authz.db.rs"]
pub mod db;
#[path = "identity.domains.authz.routes.rs"]
pub mod routes;
#[path = "identity.domains.authz.service.rs"]
pub mod service;
#[path = "identity.domains.authz.types.rs"]
pub mod types;

pub use db::target_role_for_member;
pub use service::{
    authorize_workspace_action, decide_workspace_action, ensure_email_verified,
    ensure_tenant_context, ensure_tenant_management_access, resolve_admin_scope,
};
pub use types::{
    AdminScope, ResourceContext, TenantManagementAuth, WorkspaceAccess, WorkspaceAction,
    WorkspaceDecision, WorkspacePolicy, WorkspaceRole, parse_action, parse_identity_role,
    parse_role,
};
