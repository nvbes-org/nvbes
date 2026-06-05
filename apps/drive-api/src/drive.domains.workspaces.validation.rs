use super::db::CurrentWorkspaceRecord;
use super::types::{NormalizedPolicy, UpdateWorkspacePolicyInput};
use crate::http::error::AppError;

pub fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut previous_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.starts_with('-') {
        slug.remove(0);
    }
    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "workspace".to_string()
    } else {
        slug
    }
}

pub fn validate_workspace_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Workspace name cannot be empty.",
        ));
    }
    if trimmed.chars().count() > 120 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Workspace name must be 120 characters or fewer.",
        ));
    }
    Ok(trimmed.to_owned())
}

pub fn parse_workspace_type(value: Option<&str>) -> Result<&'static str, AppError> {
    match value.unwrap_or("team").trim() {
        "personal" => Ok("personal"),
        "team" => Ok("team"),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "workspace_type must be personal or team.",
        )),
    }
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
