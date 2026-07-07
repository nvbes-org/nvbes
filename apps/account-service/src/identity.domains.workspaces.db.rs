use crate::domains::cloud::workspace_port::CloudWorkspaceSummary;

use super::types::*;

pub struct CurrentWorkspaceRecord {
    pub name: String,
    pub plan_max_share_link_ttl_days: i32,
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
    pub mfa_policy: String,
}

pub fn workspace_response_from_cloud(
    workspace: CloudWorkspaceSummary,
    role: &str,
) -> WorkspaceResponse {
    WorkspaceResponse {
        workspace: WorkspaceView {
            id: workspace.workspace_id,
            owner_principal_id: workspace.owner_principal_id.unwrap_or_default(),
            name: workspace.name,
            workspace_type: workspace.workspace_type,
            data_region: workspace.data_region.unwrap_or_default(),
            jurisdiction: workspace.jurisdiction.unwrap_or_default(),
            role: role.to_string(),
            plan_code: workspace.plan_code,
            trial_ends_at: workspace.trial_ends_at,
            policy: WorkspacePolicyView {
                member_can_create_share_links: workspace.member_can_create_share_links,
                require_admin_approval_for_member_share: workspace
                    .require_admin_approval_for_member_share,
                default_share_link_ttl_days: workspace.default_share_link_ttl_days,
                max_share_link_ttl_days: workspace.max_share_link_ttl_days,
                mfa_policy: parse_mfa_policy_setting(workspace.mfa_policy.as_deref()),
            },
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
        },
    }
}

pub fn current_workspace_from_cloud(workspace: &CloudWorkspaceSummary) -> CurrentWorkspaceRecord {
    CurrentWorkspaceRecord {
        name: workspace.name.clone(),
        plan_max_share_link_ttl_days: workspace.max_share_link_ttl_days,
        member_can_create_share_links: workspace.member_can_create_share_links,
        require_admin_approval_for_member_share: workspace.require_admin_approval_for_member_share,
        default_share_link_ttl_days: workspace.default_share_link_ttl_days,
        max_share_link_ttl_days: workspace.max_share_link_ttl_days,
        mfa_policy: workspace
            .mfa_policy
            .clone()
            .unwrap_or_else(|| "optional".to_string()),
    }
}

fn parse_mfa_policy_setting(value: Option<&str>) -> MfaPolicySetting {
    match value {
        Some("required_admins") => MfaPolicySetting::RequiredAdmins,
        Some("required_all") => MfaPolicySetting::RequiredAll,
        _ => MfaPolicySetting::Optional,
    }
}
