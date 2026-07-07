use super::db;
use super::policy::*;
use super::types::*;
use super::validation::*;
use crate::{
    domains::{authz::WorkspaceAccess, cloud::workspace_port},
    http::error::AppError,
};
use nvbes_product_account::cloud_boundary::{
    UpdateWorkspaceSettingsCommand, WorkspacePolicyCommand,
};
use sqlx::PgPool;

pub async fn update_workspace(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: UpdateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    let current_workspace =
        workspace_port::get_workspace(access.tenant_id, access.workspace_id, access.auth.user_id)
            .await?;
    let current = db::current_workspace_from_cloud(&current_workspace);
    let previous_name = current.name.clone();
    let name = match input.name {
        Some(name) => validate_workspace_name(&name)?,
        None => previous_name.clone(),
    };

    let policy = normalize_policy_update(&current, input.policy)?;
    let mut tx = db.begin().await?;

    crate::domains::cloud::workspace_port::update_workspace_settings_tx(
        &mut tx,
        &UpdateWorkspaceSettingsCommand {
            actor_principal_id: access.auth.user_id,
            workspace_id: access.workspace_id,
            name: name.clone(),
            policy: WorkspacePolicyCommand {
                member_can_create_share_links: policy.member_can_create_share_links,
                require_admin_approval_for_member_share: policy
                    .require_admin_approval_for_member_share,
                default_share_link_ttl_days: policy.default_share_link_ttl_days,
                max_share_link_ttl_days: policy.max_share_link_ttl_days,
                required_acr: None,
                mfa_policy: Some(policy.mfa_policy.as_str().to_string()),
            },
        },
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id: access.tenant_id.unwrap_or_default(),
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
            action: "workspace.updated",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "name_changed": previous_name != name,
                "policy": {
                    "member_can_create_share_links": policy.member_can_create_share_links,
                    "require_admin_approval_for_member_share": policy.require_admin_approval_for_member_share,
                    "default_share_link_ttl_days": policy.default_share_link_ttl_days,
                    "max_share_link_ttl_days": policy.max_share_link_ttl_days,
                    "mfa_policy": policy.mfa_policy.as_str(),
                }
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let workspace =
        workspace_port::get_workspace(access.tenant_id, access.workspace_id, access.auth.user_id)
            .await?;
    Ok(db::workspace_response_from_cloud(
        workspace,
        nvbes_tenancy::role_as_str(access.role),
    ))
}
