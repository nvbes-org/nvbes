use super::db::CurrentWorkspaceRecord;
use super::types::UpdateWorkspacePolicyInput;
use crate::http::error::AppError;

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
    let Some(input) = input else {
        return Ok(NormalizedPolicy {
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
        return Err(AppError::bad_request(
            "validation_failed",
            "Share link TTL values must be greater than zero.",
        ));
    }
    if max_share_link_ttl_days > current.plan_max_share_link_ttl_days {
        return Err(AppError::bad_request(
            "validation_failed",
            "Workspace share link TTL cannot exceed the current plan maximum.",
        ));
    }
    if default_share_link_ttl_days > max_share_link_ttl_days {
        return Err(AppError::bad_request(
            "validation_failed",
            "Default share link TTL cannot exceed the workspace maximum TTL.",
        ));
    }

    Ok(NormalizedPolicy {
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
