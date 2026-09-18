use uuid::Uuid;

use nvbes_core::authz::WorkspaceRole;
use thiserror::Error;

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

#[derive(Debug, Clone, Copy)]
pub struct CurrentWorkspacePolicy {
    pub plan_max_share_link_ttl_days: i32,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspacePolicyUpdate {
    pub member_can_create_share_links: Option<bool>,
    pub require_admin_approval_for_member_share: Option<bool>,
    pub default_share_link_ttl_days: Option<i32>,
    pub max_share_link_ttl_days: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedWorkspacePolicy {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkspacePolicyError {
    #[error("Share link TTL values must be greater than zero.")]
    ShareLinkTtlNonPositive,
    #[error("Workspace share link TTL cannot exceed the current plan maximum.")]
    ShareLinkTtlExceedsPlan,
    #[error("Default share link TTL cannot exceed the workspace maximum TTL.")]
    DefaultShareLinkTtlExceedsMax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkspaceInputError {
    #[error("Workspace name cannot be empty.")]
    WorkspaceNameEmpty,
    #[error("Workspace name must be 120 characters or fewer.")]
    WorkspaceNameTooLong,
    #[error("workspace_type must be personal or team.")]
    WorkspaceTypeInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkspaceAccessError {
    #[error("You do not have access to this workspace.")]
    AccessDenied,
    #[error("Switch to this workspace before performing the requested action.")]
    ContextMismatch,
    #[error("The principal does not expose a workspace role.")]
    PrincipalRoleMissing,
    #[error("The delegated actor does not have access to this workspace.")]
    DelegatedAccessDenied,
    #[error("The delegated actor does not match this workspace.")]
    DelegatedContextMismatch,
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

pub fn normalize_workspace_policy_update(
    current: CurrentWorkspacePolicy,
    input: Option<WorkspacePolicyUpdate>,
) -> Result<NormalizedWorkspacePolicy, WorkspacePolicyError> {
    let Some(input) = input else {
        return Ok(NormalizedWorkspacePolicy {
            member_can_create_share_links: current.member_can_create_share_links,
            require_admin_approval_for_member_share: current
                .require_admin_approval_for_member_share,
            default_share_link_ttl_days: current.default_share_link_ttl_days,
            max_share_link_ttl_days: current.max_share_link_ttl_days,
        });
    };

    let max_share_link_ttl_days = input
        .max_share_link_ttl_days
        .unwrap_or(current.max_share_link_ttl_days);
    let default_share_link_ttl_days = input
        .default_share_link_ttl_days
        .unwrap_or(current.default_share_link_ttl_days);

    if max_share_link_ttl_days <= 0 || default_share_link_ttl_days <= 0 {
        return Err(WorkspacePolicyError::ShareLinkTtlNonPositive);
    }
    if max_share_link_ttl_days > current.plan_max_share_link_ttl_days {
        return Err(WorkspacePolicyError::ShareLinkTtlExceedsPlan);
    }
    if default_share_link_ttl_days > max_share_link_ttl_days {
        return Err(WorkspacePolicyError::DefaultShareLinkTtlExceedsMax);
    }

    Ok(NormalizedWorkspacePolicy {
        member_can_create_share_links: input
            .member_can_create_share_links
            .unwrap_or(current.member_can_create_share_links),
        require_admin_approval_for_member_share: input
            .require_admin_approval_for_member_share
            .unwrap_or(current.require_admin_approval_for_member_share),
        default_share_link_ttl_days,
        max_share_link_ttl_days,
    })
}

pub fn validate_workspace_name(name: &str) -> Result<String, WorkspaceInputError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(WorkspaceInputError::WorkspaceNameEmpty);
    }
    if trimmed.chars().count() > 120 {
        return Err(WorkspaceInputError::WorkspaceNameTooLong);
    }
    Ok(trimmed.to_owned())
}

pub fn parse_workspace_type(value: Option<&str>) -> Result<&'static str, WorkspaceInputError> {
    match value.unwrap_or("team").trim() {
        "personal" => Ok("personal"),
        "team" => Ok("team"),
        _ => Err(WorkspaceInputError::WorkspaceTypeInvalid),
    }
}

pub fn role_as_db(role: WorkspaceRole) -> &'static str {
    match role {
        WorkspaceRole::Owner => "owner",
        WorkspaceRole::Admin => "admin",
        WorkspaceRole::SecurityAdmin => "security_admin",
        WorkspaceRole::BillingAdmin => "billing_admin",
        WorkspaceRole::Member => "member",
        WorkspaceRole::Viewer => "viewer",
    }
}

pub fn role_as_str(role: WorkspaceRole) -> &'static str {
    role_as_db(role)
}

#[cfg(test)]
mod tests {
    use super::{
        CurrentWorkspacePolicy, NormalizedWorkspacePolicy, WorkspaceInputError,
        WorkspacePolicyError, WorkspacePolicyUpdate, normalize_workspace_policy_update,
    };

    fn current_policy() -> CurrentWorkspacePolicy {
        CurrentWorkspacePolicy {
            plan_max_share_link_ttl_days: 90,
            member_can_create_share_links: true,
            require_admin_approval_for_member_share: false,
            default_share_link_ttl_days: 14,
            max_share_link_ttl_days: 30,
        }
    }

    #[test]
    fn normalize_workspace_policy_update_preserves_current_policy_when_missing() {
        let normalized = normalize_workspace_policy_update(current_policy(), None)
            .expect("missing input should preserve current policy");

        assert_eq!(
            normalized,
            NormalizedWorkspacePolicy {
                member_can_create_share_links: true,
                require_admin_approval_for_member_share: false,
                default_share_link_ttl_days: 14,
                max_share_link_ttl_days: 30,
            }
        );
    }

    #[test]
    fn normalize_workspace_policy_update_rejects_non_positive_ttl() {
        let error = normalize_workspace_policy_update(
            current_policy(),
            Some(WorkspacePolicyUpdate {
                default_share_link_ttl_days: Some(0),
                ..WorkspacePolicyUpdate::default()
            }),
        )
        .expect_err("zero ttl should be rejected");

        assert_eq!(error, WorkspacePolicyError::ShareLinkTtlNonPositive);
    }

    #[test]
    fn validate_workspace_name_rejects_blank_names() {
        let error = super::validate_workspace_name("   ")
            .expect_err("blank workspace names should be rejected");

        assert_eq!(error, WorkspaceInputError::WorkspaceNameEmpty);
    }

    #[test]
    fn parse_workspace_type_accepts_known_values() {
        assert_eq!(
            super::parse_workspace_type(Some(" personal ")).expect("personal should be accepted"),
            "personal"
        );
        assert_eq!(
            super::parse_workspace_type(None).expect("default should be team"),
            "team"
        );
    }
}
