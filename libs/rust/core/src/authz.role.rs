use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum WorkspaceRole {
    Owner,
    Admin,
    SecurityAdmin,
    BillingAdmin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum IdentityRole {
    Owner,
    Admin,
    SecurityAdmin,
    BillingAdmin,
    Member,
}

impl std::fmt::Display for WorkspaceRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceRole::Owner => write!(f, "owner"),
            WorkspaceRole::Admin => write!(f, "admin"),
            WorkspaceRole::SecurityAdmin => write!(f, "security_admin"),
            WorkspaceRole::BillingAdmin => write!(f, "billing_admin"),
            WorkspaceRole::Member => write!(f, "member"),
            WorkspaceRole::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::fmt::Display for IdentityRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityRole::Owner => write!(f, "owner"),
            IdentityRole::Admin => write!(f, "admin"),
            IdentityRole::SecurityAdmin => write!(f, "security_admin"),
            IdentityRole::BillingAdmin => write!(f, "billing_admin"),
            IdentityRole::Member => write!(f, "member"),
        }
    }
}

pub fn parse_role(role: &str) -> Option<WorkspaceRole> {
    match role {
        "owner" => Some(WorkspaceRole::Owner),
        "admin" => Some(WorkspaceRole::Admin),
        "security_admin" => Some(WorkspaceRole::SecurityAdmin),
        "billing_admin" => Some(WorkspaceRole::BillingAdmin),
        "member" => Some(WorkspaceRole::Member),
        "viewer" => Some(WorkspaceRole::Viewer),
        _ => None,
    }
}

pub fn parse_identity_role(role: &str) -> Option<IdentityRole> {
    match role {
        "owner" => Some(IdentityRole::Owner),
        "admin" => Some(IdentityRole::Admin),
        "security_admin" => Some(IdentityRole::SecurityAdmin),
        "billing_admin" => Some(IdentityRole::BillingAdmin),
        "member" => Some(IdentityRole::Member),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{IdentityRole, WorkspaceRole, parse_identity_role, parse_role};

    #[test]
    fn parses_identity_admin_roles() {
        assert_eq!(parse_identity_role("owner"), Some(IdentityRole::Owner));
        assert_eq!(parse_identity_role("admin"), Some(IdentityRole::Admin));
        assert_eq!(
            parse_identity_role("security_admin"),
            Some(IdentityRole::SecurityAdmin)
        );
        assert_eq!(
            parse_identity_role("billing_admin"),
            Some(IdentityRole::BillingAdmin)
        );
        assert_eq!(parse_identity_role("member"), Some(IdentityRole::Member));
    }

    #[test]
    fn parses_legacy_workspace_viewer_role() {
        assert_eq!(parse_role("viewer"), Some(WorkspaceRole::Viewer));
    }
}
