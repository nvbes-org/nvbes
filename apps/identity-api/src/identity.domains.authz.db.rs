use super::types::*;
use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;
use crate::http::request::{client_ip, user_agent};
use axum::http::HeaderMap;
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
            "Switch to this workspace before performing the requested action.",
        ));
    }

    let row = sqlx::query(
        r#"
        SELECT
          w.tenant_id,
          w.organization_id,
          wm.role::text AS role,
          wp.member_can_create_share_links
        FROM workspaces w
        INNER JOIN workspace_memberships wm ON wm.workspace_id = w.id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
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
            "You do not have access to this workspace.",
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
        return Err(AppError::unauthorized(
            "step_up_required",
            "Please verify again before continuing.",
        ));
    }

    Ok(WorkspaceAccess {
        auth: auth.clone(),
        workspace_id,
        tenant_id,
        organization_id,
        role: parse_role(row.get::<String, _>("role").as_str())?,
        policy: WorkspacePolicy::member_share_links_enabled(
            row.get("member_can_create_share_links"),
        ),
    })
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
