use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl std::fmt::Display for WorkspaceRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceRole::Owner => write!(f, "owner"),
            WorkspaceRole::Admin => write!(f, "admin"),
            WorkspaceRole::Member => write!(f, "member"),
            WorkspaceRole::Viewer => write!(f, "viewer"),
        }
    }
}

pub fn parse_role(role: &str) -> Option<WorkspaceRole> {
    match role {
        "owner" => Some(WorkspaceRole::Owner),
        "admin" => Some(WorkspaceRole::Admin),
        "member" => Some(WorkspaceRole::Member),
        "viewer" => Some(WorkspaceRole::Viewer),
        _ => None,
    }
}
