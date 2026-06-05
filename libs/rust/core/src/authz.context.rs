use super::role::WorkspaceRole;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContext {
    pub owns_resource: bool,
    pub member_share_links_enabled: bool,
    pub target_role: Option<WorkspaceRole>,
}
