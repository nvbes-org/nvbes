use super::db::CurrentWorkspaceRecord;
use super::types::{NormalizedPolicy, UpdateWorkspacePolicyInput};
use crate::http::error::AppError;
use nvbes_tenancy::workspace::{
    CurrentWorkspacePolicy, NormalizedWorkspacePolicy, WorkspaceInputError, WorkspacePolicyError,
    WorkspacePolicyUpdate, normalize_workspace_policy_update,
};

pub fn slugify(value: &str) -> String {
    let slug = nvbes_core::auth::slugify(value);
    if slug.is_empty() || slug == "tenant" {
        "workspace".to_string()
    } else {
        slug
    }
}

pub fn validate_workspace_name(name: &str) -> Result<String, AppError> {
    nvbes_tenancy::workspace::validate_workspace_name(name).map_err(map_workspace_input_error)
}

pub fn parse_workspace_type(value: Option<&str>) -> Result<&'static str, AppError> {
    nvbes_tenancy::workspace::parse_workspace_type(value).map_err(map_workspace_input_error)
}

pub fn normalize_policy_update(
    current: &CurrentWorkspaceRecord,
    input: Option<UpdateWorkspacePolicyInput>,
) -> Result<NormalizedPolicy, AppError> {
    let normalized = normalize_workspace_policy_update(
        CurrentWorkspacePolicy {
            plan_max_share_link_ttl_days: current.plan_max_share_link_ttl_days,
            member_can_create_share_links: current.member_can_create_share_links,
            require_admin_approval_for_member_share: current
                .require_admin_approval_for_member_share,
            default_share_link_ttl_days: current.default_share_link_ttl_days,
            max_share_link_ttl_days: current.max_share_link_ttl_days,
        },
        input.map(|input| WorkspacePolicyUpdate {
            member_can_create_share_links: input.member_can_create_share_links,
            require_admin_approval_for_member_share: input.require_admin_approval_for_member_share,
            default_share_link_ttl_days: input.default_share_link_ttl_days,
            max_share_link_ttl_days: input.max_share_link_ttl_days,
        }),
    )
    .map_err(map_policy_error)?;

    Ok(from_shared_policy(normalized))
}

fn from_shared_policy(policy: NormalizedWorkspacePolicy) -> NormalizedPolicy {
    NormalizedPolicy {
        member_can_create_share_links: policy.member_can_create_share_links,
        require_admin_approval_for_member_share: policy.require_admin_approval_for_member_share,
        default_share_link_ttl_days: policy.default_share_link_ttl_days,
        max_share_link_ttl_days: policy.max_share_link_ttl_days,
    }
}

fn map_policy_error(error: WorkspacePolicyError) -> AppError {
    AppError::bad_request("validation_failed", error.to_string())
}

fn map_workspace_input_error(error: WorkspaceInputError) -> AppError {
    AppError::bad_request("validation_failed", error.to_string())
}
