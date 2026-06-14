use super::db;
use super::policy::*;
use super::types::*;
use super::validation::*;
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use sqlx::PgPool;

pub async fn update_workspace(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: UpdateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    let current = db::fetch_workspace_for_update(db, access.workspace_id).await?;
    let previous_name = current.name.clone();
    let name = match input.name {
        Some(name) => validate_workspace_name(&name)?,
        None => previous_name.clone(),
    };

    let policy = normalize_policy_update(&current, input.policy)?;
    let mut tx = db.begin().await?;

    sqlx::query(
        r#"
        UPDATE workspaces
        SET name = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(access.workspace_id)
    .bind(&name)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_policies
        SET member_can_create_share_links = $2,
            require_admin_approval_for_member_share = $3,
            default_share_link_ttl_days = $4,
            max_share_link_ttl_days = $5,
            mfa_policy = $6,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(access.workspace_id)
    .bind(policy.member_can_create_share_links)
    .bind(policy.require_admin_approval_for_member_share)
    .bind(policy.default_share_link_ttl_days)
    .bind(policy.max_share_link_ttl_days)
    .bind(policy.mfa_policy.as_str())
    .execute(&mut *tx)
    .await?;

    nvbes_audit::insert_audit_event_tx(
        &mut tx,
        nvbes_audit::AuditEventInput {
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

    db::get_workspace_by_id(
        db,
        access.workspace_id,
        nvbes_tenancy::role_as_str(access.role),
    )
    .await
}
