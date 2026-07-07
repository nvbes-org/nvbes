use super::types::*;
use crate::domains::auth::types::AuthContext;
use crate::domains::cloud::workspace_port;
use crate::http::error::AppError;
use crate::http::request::{client_ip, user_agent};
use axum::http::HeaderMap;
use nvbes_tenancy::{
    InheritedPolicyLayer, resolve_inherited_policy, workspace::WorkspaceAccessError,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn load_workspace_access(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    workspace_id: Uuid,
) -> Result<WorkspaceAccess, AppError> {
    if auth
        .workspace_id
        .is_some_and(|current_workspace| current_workspace != workspace_id)
    {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            WorkspaceAccessError::ContextMismatch.to_string(),
        ));
    }

    let workspace =
        workspace_port::get_workspace(auth.tenant_id, workspace_id, auth.user_id).await?;
    let role = workspace_port::list_workspace_members(
        Some(workspace.tenant_id),
        workspace_id,
        auth.user_id,
    )
    .await?
    .into_iter()
    .find(|member| member.principal_id == auth.user_id && member.active)
    .map(|member| member.role)
    .ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            WorkspaceAccessError::AccessDenied.to_string(),
        )
    })?;

    let tenant_id = Some(workspace.tenant_id);
    let organization_id = workspace.organization_id;

    let policy_row = sqlx::query(
        r#"
        SELECT
          sp.member_can_create_share_links AS system_member_can_create_share_links,
          sp.require_admin_approval_for_member_share AS system_require_admin_approval_for_member_share,
          sp.default_share_link_ttl_days AS system_default_share_link_ttl_days,
          sp.max_share_link_ttl_days AS system_max_share_link_ttl_days,
          tp.member_can_create_share_links AS tenant_member_can_create_share_links,
          tp.require_admin_approval_for_member_share AS tenant_require_admin_approval_for_member_share,
          tp.default_share_link_ttl_days AS tenant_default_share_link_ttl_days,
          tp.max_share_link_ttl_days AS tenant_max_share_link_ttl_days,
          op.member_can_create_share_links AS organization_member_can_create_share_links,
          op.require_admin_approval_for_member_share AS organization_require_admin_approval_for_member_share,
          op.default_share_link_ttl_days AS organization_default_share_link_ttl_days,
          op.max_share_link_ttl_days AS organization_max_share_link_ttl_days
        FROM system_policies sp
        LEFT JOIN tenant_policies tp ON tp.tenant_id = $1
        LEFT JOIN organization_policies op ON op.organization_id = $2
        WHERE sp.id = TRUE
        "#,
    )
    .bind(workspace.tenant_id)
    .bind(workspace.organization_id)
    .fetch_one(db)
    .await?;
    let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
        db,
        redis,
        auth.user_id,
        Some(auth.session_id),
        auth.client_id.as_deref(),
        tenant_id,
        organization_id,
        Some(workspace_id),
    )
    .await?;
    if !assurance.sufficient {
        return Err(nvbes_core::auth::step_up_required_error().into());
    }
    let effective_policy = resolve_inherited_policy([
        InheritedPolicyLayer::system()
            .with_member_share_links(policy_row.get("system_member_can_create_share_links"))
            .with_require_admin_approval_for_member_share(
                policy_row.get("system_require_admin_approval_for_member_share"),
            )
            .with_default_share_link_ttl_days(policy_row.get("system_default_share_link_ttl_days"))
            .with_max_share_link_ttl_days(policy_row.get("system_max_share_link_ttl_days")),
        optional_policy_layer(
            InheritedPolicyLayer::tenant(),
            policy_row.get("tenant_member_can_create_share_links"),
            policy_row.get("tenant_require_admin_approval_for_member_share"),
            policy_row.get("tenant_default_share_link_ttl_days"),
            policy_row.get("tenant_max_share_link_ttl_days"),
        ),
        optional_policy_layer(
            InheritedPolicyLayer::organization(),
            policy_row.get("organization_member_can_create_share_links"),
            policy_row.get("organization_require_admin_approval_for_member_share"),
            policy_row.get("organization_default_share_link_ttl_days"),
            policy_row.get("organization_max_share_link_ttl_days"),
        ),
        InheritedPolicyLayer::workspace()
            .with_member_share_links(workspace.member_can_create_share_links)
            .with_require_admin_approval_for_member_share(
                workspace.require_admin_approval_for_member_share,
            )
            .with_default_share_link_ttl_days(workspace.default_share_link_ttl_days)
            .with_max_share_link_ttl_days(workspace.max_share_link_ttl_days),
    ]);

    Ok(WorkspaceAccess {
        auth: auth.clone(),
        workspace_id,
        tenant_id,
        organization_id,
        role: parse_role(role.as_str())?,
        policy: WorkspacePolicy::member_share_links_enabled(
            effective_policy.member_can_create_share_links,
        ),
    })
}

fn optional_policy_layer(
    mut layer: InheritedPolicyLayer,
    member_can_create_share_links: Option<bool>,
    require_admin_approval_for_member_share: Option<bool>,
    default_share_link_ttl_days: Option<i32>,
    max_share_link_ttl_days: Option<i32>,
) -> InheritedPolicyLayer {
    layer.member_can_create_share_links = member_can_create_share_links;
    layer.require_admin_approval_for_member_share = require_admin_approval_for_member_share;
    layer.default_share_link_ttl_days = default_share_link_ttl_days;
    layer.max_share_link_ttl_days = max_share_link_ttl_days;
    layer
}

pub async fn record_permission_denied(
    db: &PgPool,
    access: &WorkspaceAccess,
    action: WorkspaceAction,
    resource: ResourceContext,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let tenant_id = if let Some(tenant_id) = access.tenant_id {
        tenant_id
    } else {
        workspace_port::get_workspace(None, access.workspace_id, access.auth.user_id)
            .await?
            .tenant_id
    };
    let mut tx = db.begin().await?;
    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: Some(access.workspace_id),
            actor_principal_id: Some(access.auth.user_id),
            action: "permission.denied",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: client_ip(headers).as_deref(),
            user_agent: user_agent(headers).as_deref(),
            metadata: permission_denied_metadata(access.role, action, resource),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn permission_denied_metadata(
    role: WorkspaceRole,
    action: WorkspaceAction,
    resource: ResourceContext,
) -> serde_json::Value {
    serde_json::json!({
      "requested_action": format!("{action:?}"),
      "role": format!("{role:?}"),
      "owns_resource": resource.owns_resource,
      "member_share_links_enabled": resource.member_share_links_enabled,
      "target_role": resource.target_role.map(|role| format!("{role:?}"))
    })
}

pub async fn target_role_for_member(
    _db: &PgPool,
    workspace_id: Uuid,
    member_id: Uuid,
) -> Result<WorkspaceRole, AppError> {
    let role = workspace_port::list_workspace_members(None, workspace_id, member_id)
        .await?
        .into_iter()
        .find(|member| member.principal_id == member_id)
        .map(|member| member.role)
        .ok_or_else(|| AppError::not_found("member_not_found", "Member not found."))?;
    parse_role(role.as_str())
}

#[cfg(test)]
mod tests {
    use super::{ResourceContext, WorkspaceAction, WorkspaceRole, permission_denied_metadata};
    use serde_json::json;

    #[test]
    fn permission_denied_metadata_captures_role_action_and_target_role() {
        let metadata = permission_denied_metadata(
            WorkspaceRole::Admin,
            WorkspaceAction::InviteMember,
            ResourceContext {
                target_role: Some(WorkspaceRole::Owner),
                owns_resource: false,
                member_share_links_enabled: true,
            },
        );

        assert_eq!(
            metadata,
            json!({
                "requested_action": "InviteMember",
                "role": "Admin",
                "owns_resource": false,
                "member_share_links_enabled": true,
                "target_role": "Owner",
            })
        );
    }
}
