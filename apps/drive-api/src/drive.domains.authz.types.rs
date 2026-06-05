use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;

pub use nvbes_core::authz::{ResourceContext, WorkspaceAction, WorkspaceRole};
pub use nvbes_tenancy::WorkspacePolicy;
pub type WorkspaceAccess = nvbes_tenancy::WorkspaceAccess<AuthContext>;

pub fn parse_role(role: &str) -> Result<WorkspaceRole, AppError> {
    nvbes_core::authz::parse_role(role).ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            format!("Unsupported workspace role: {role}."),
        )
    })
}
