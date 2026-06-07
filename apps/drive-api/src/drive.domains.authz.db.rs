#[path = "drive.domains.authz.db.access.rs"]
mod access;
#[path = "drive.domains.authz.db.audit.rs"]
mod audit;
#[path = "drive.domains.authz.db.context.rs"]
mod context;
#[path = "drive.domains.authz.db.role_policy.rs"]
mod role_policy;
#[cfg(test)]
#[path = "drive.domains.authz.db.tests.rs"]
mod tests;
#[path = "drive.domains.authz.db.workspace_scope.rs"]
mod workspace_scope;

pub use access::load_workspace_access;
pub use audit::record_permission_denied;
pub use context::{
    resource_context_for_object, resource_context_for_share_link, target_role_for_member,
};
