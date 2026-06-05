use super::role::WorkspaceRole;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceDecision {
    pub allowed: bool,
    pub reason: String,
    pub action: String,
    pub role: Option<WorkspaceRole>,
    pub requires_step_up: bool,
}
