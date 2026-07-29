#[path = "authz.action.rs"]
mod action;
#[path = "authz.context.rs"]
mod context;
#[path = "authz.decision.rs"]
mod decision;
#[path = "authz.policy.rs"]
mod policy;
#[path = "authz.role.rs"]
mod role;

pub use action::{
    WorkspaceAction, action_requires_independent_approval, action_requires_step_up, parse_action,
};
pub use context::ResourceContext;
pub use decision::WorkspaceDecision;
pub use policy::is_allowed;
pub use role::{IdentityRole, WorkspaceRole, parse_identity_role, parse_role};
