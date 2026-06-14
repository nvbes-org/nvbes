use super::types::*;
use crate::domains::auth::types::AuthContext;
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

    let row = sqlx::query(
        r#"
        SELECT
          w.tenant_id,
          w.organization_id,
          wm.role::text AS role,
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
          op.max_share_link_ttl_days AS organization_max_share_link_ttl_days,
          wp.member_can_create_share_links AS workspace_member_can_create_share_links,
          wp.require_admin_approval_for_member_share AS workspace_require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days AS workspace_default_share_link_ttl_days,
          wp.max_share_link_ttl_days AS workspace_max_share_link_ttl_days
        FROM workspaces w
        INNER JOIN workspace_memberships wm ON wm.workspace_id = w.id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        INNER JOIN system_policies sp ON sp.id = TRUE
        LEFT JOIN tenant_policies tp ON tp.tenant_id = w.tenant_id
        LEFT JOIN organization_policies op ON op.organization_id = w.organization_id
        WHERE wm.workspace_id = $1
          AND wm.principal_id = $2
          AND wm.status = 'active'
        "#,
    )
    .bind(workspace_id)
    .bind(auth.user_id)
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            WorkspaceAccessError::AccessDenied.to_string(),
        )
    })?;

    let tenant_id: Option<Uuid> = row.get("tenant_id");
    let organization_id: Option<Uuid> = row.get("organization_id");
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
            .with_member_share_links(row.get("system_member_can_create_share_links"))
            .with_require_admin_approval_for_member_share(
                row.get("system_require_admin_approval_for_member_share"),
            )
            .with_default_share_link_ttl_days(row.get("system_default_share_link_ttl_days"))
            .with_max_share_link_ttl_days(row.get("system_max_share_link_ttl_days")),
        optional_policy_layer(
            InheritedPolicyLayer::tenant(),
            row.get("tenant_member_can_create_share_links"),
            row.get("tenant_require_admin_approval_for_member_share"),
            row.get("tenant_default_share_link_ttl_days"),
            row.get("tenant_max_share_link_ttl_days"),
        ),
        optional_policy_layer(
            InheritedPolicyLayer::organization(),
            row.get("organization_member_can_create_share_links"),
            row.get("organization_require_admin_approval_for_member_share"),
            row.get("organization_default_share_link_ttl_days"),
            row.get("organization_max_share_link_ttl_days"),
        ),
        InheritedPolicyLayer::workspace()
            .with_member_share_links(row.get("workspace_member_can_create_share_links"))
            .with_require_admin_approval_for_member_share(
                row.get("workspace_require_admin_approval_for_member_share"),
            )
            .with_default_share_link_ttl_days(row.get("workspace_default_share_link_ttl_days"))
            .with_max_share_link_ttl_days(row.get("workspace_max_share_link_ttl_days")),
    ]);

    Ok(WorkspaceAccess {
        auth: auth.clone(),
        workspace_id,
        tenant_id,
        organization_id,
        role: parse_role(row.get::<String, _>("role").as_str())?,
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
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, 'permission.denied', 'workspace', $3, $4::inet, $5, $6)
        "#,
    )
    .bind(access.workspace_id)
    .bind(access.auth.user_id)
    .bind(access.workspace_id)
    .bind(client_ip(headers).as_deref())
    .bind(user_agent(headers).as_deref())
    .bind(sqlx::types::Json(serde_json::json!({
      "requested_action": format!("{action:?}"),
      "role": format!("{:?}", access.role),
      "owns_resource": resource.owns_resource,
      "member_share_links_enabled": resource.member_share_links_enabled,
      "target_role": resource.target_role.map(|role| format!("{role:?}"))
    })))
    .execute(db)
    .await?;

    Ok(())
}

pub async fn target_role_for_member(
    db: &PgPool,
    workspace_id: Uuid,
    member_id: Uuid,
) -> Result<WorkspaceRole, AppError> {
    let row = sqlx::query(
        r#"
        SELECT role::text AS role
        FROM workspace_memberships
        WHERE workspace_id = $1 AND principal_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| AppError::not_found("member_not_found", "Member not found."))?;
    parse_role(row.get::<String, _>("role").as_str())
}
