use super::db::CurrentWorkspaceRecord;
use super::types::UpdateWorkspacePolicyInput;
use crate::http::error::AppError;
use nvbes_tenancy::workspace::{
    CurrentWorkspacePolicy, NormalizedWorkspacePolicy, WorkspacePolicyError, WorkspacePolicyUpdate,
    normalize_workspace_policy_update,
};

pub struct NormalizedPolicy {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
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
