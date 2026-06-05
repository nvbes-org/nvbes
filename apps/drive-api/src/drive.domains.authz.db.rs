use axum::http::HeaderMap;
use sqlx::Row;
use uuid::Uuid;

use super::types::{
    ResourceContext, WorkspaceAccess, WorkspaceAction, WorkspacePolicy, WorkspaceRole, parse_role,
};
use crate::{
    domains::auth::types::{AuthContext, AuthPrincipalKind},
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

pub async fn load_workspace_access(
    db: &sqlx::PgPool,
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

    let row = if auth.principal_kind == AuthPrincipalKind::ServiceAccount || auth.role.is_some() {
        let token_role = auth.role.as_deref().ok_or_else(|| {
            AppError::forbidden(
                "workspace_access_denied",
                "The principal does not expose a workspace role.",
            )
        })?;
        let row = sqlx::query(
            r#"
            SELECT
              w.tenant_id,
              w.organization_id,
              wp.member_can_create_share_links
            FROM workspaces w
            INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
            WHERE w.id = $1
              AND w.deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
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
        validate_workspace_context(auth, tenant_id, organization_id, workspace_id)?;
        validate_actor_context(auth, tenant_id, organization_id, workspace_id)?;

        let parsed_role = parse_role(token_role)?;
        let effective_role = auth
            .actor
            .as_ref()
            .and_then(|actor| actor.role.as_deref())
            .map(parse_role)
            .transpose()?
            .map(|actor_role| narrow_role(parsed_role, actor_role))
            .unwrap_or(parsed_role);

        return Ok(WorkspaceAccess {
            auth: auth.clone(),
            workspace_id,
            tenant_id,
            organization_id,
            role: effective_role,
            policy: WorkspacePolicy::member_share_links_enabled(
                row.get("member_can_create_share_links"),
            ),
        });
    } else {
        sqlx::query(
            r#"
            SELECT
              wm.role::text AS role,
              w.tenant_id,
              w.organization_id,
              wp.member_can_create_share_links
            FROM workspace_memberships wm
            INNER JOIN workspace_policies wp ON wp.workspace_id = wm.workspace_id
            INNER JOIN workspaces w ON w.id = wm.workspace_id
            WHERE wm.workspace_id = $1
              AND wm.user_id = $2
              AND wm.status = 'active'
              AND w.deleted_at IS NULL
            "#,
        )
        .bind(workspace_id)
        .bind(auth.user_id()?)
        .fetch_optional(db)
        .await?
    };

    let row = row.ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            "You do not have access to this workspace.",
        )
    })?;

    let tenant_id: Option<Uuid> = row.get("tenant_id");
    let organization_id: Option<Uuid> = row.get("organization_id");
    validate_workspace_context(auth, tenant_id, organization_id, workspace_id)?;
    validate_actor_context(auth, tenant_id, organization_id, workspace_id)?;

    let user_role = parse_role(row.get::<String, _>("role").as_str())?;
    let effective_role = auth
        .actor
        .as_ref()
        .and_then(|actor| actor.role.as_deref())
        .map(parse_role)
        .transpose()?
        .map(|actor_role| narrow_role(user_role, actor_role))
        .unwrap_or(user_role);

    Ok(WorkspaceAccess {
        auth: auth.clone(),
        workspace_id,
        tenant_id,
        organization_id,
        role: effective_role,
        policy: WorkspacePolicy::member_share_links_enabled(
            row.get("member_can_create_share_links"),
        ),
    })
}

pub async fn record_permission_denied(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    action: WorkspaceAction,
    resource: ResourceContext,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, 'permission.denied', 'workspace', $4, $5::inet, $6, $7)
        "#,
    )
    .bind(access.workspace_id)
    .bind(access.auth.audit_actor_user_id())
    .bind(Some(access.auth.audit_actor_principal_id()))
    .bind(access.workspace_id)
    .bind(client_ip(headers).as_deref())
    .bind(user_agent(headers).as_deref())
    .bind(sqlx::types::Json(serde_json::json!({
      "requested_action": format!("{action:?}"),
      "role": format!("{:?}", access.role),
      "principal_id": access.auth.principal_id,
      "principal_type": match access.auth.principal_kind {
        AuthPrincipalKind::User => "user",
        AuthPrincipalKind::ServiceAccount => "service_account",
      },
      "tenant_id": access.tenant_id,
      "organization_id": access.organization_id,
      "owns_resource": resource.owns_resource,
      "member_share_links_enabled": resource.member_share_links_enabled,
      "target_role": resource.target_role.map(|role| format!("{role:?}")),
      "actor": access.auth.actor.as_ref().map(|actor| serde_json::json!({
        "principal_id": actor.principal_id,
        "principal_type": match actor.principal_kind {
          AuthPrincipalKind::User => "user",
          AuthPrincipalKind::ServiceAccount => "service_account",
        },
        "tenant_id": actor.tenant_id,
        "organization_id": actor.organization_id,
        "workspace_id": actor.workspace_id,
        "role": actor.role,
        "client_id": actor.client_id
      }))
    })))
    .execute(db)
    .await?;

    Ok(())
}

pub async fn resource_context_for_object(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    actor_user_id: Uuid,
    actor_principal_id: Uuid,
    object_id: Uuid,
) -> Result<ResourceContext, AppError> {
    let row = sqlx::query(
        r#"
        SELECT created_by, created_by_principal_id
        FROM storage_objects
        WHERE id = $1 AND workspace_id = $2
        "#,
    )
    .bind(object_id)
    .bind(workspace_id)
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| AppError::not_found("object_not_found", "Object not found."))?;
    let created_by: Uuid = row.get("created_by");
    let created_by_principal_id: Uuid = row.get("created_by_principal_id");

    Ok(ResourceContext {
        owns_resource: owns_resource(
            created_by,
            created_by_principal_id,
            actor_user_id,
            actor_principal_id,
        ),
        ..ResourceContext::default()
    })
}

pub async fn resource_context_for_share_link(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    actor_user_id: Uuid,
    actor_principal_id: Uuid,
    share_link_id: Uuid,
) -> Result<ResourceContext, AppError> {
    let row = sqlx::query(
        r#"
        SELECT so.created_by, so.created_by_principal_id, wp.member_can_create_share_links
        FROM share_links sl
        INNER JOIN storage_objects so ON so.id = sl.storage_object_id
        INNER JOIN workspace_policies wp ON wp.workspace_id = sl.workspace_id
        WHERE sl.id = $1 AND sl.workspace_id = $2
        "#,
    )
    .bind(share_link_id)
    .bind(workspace_id)
    .fetch_optional(db)
    .await?;

    let row =
        row.ok_or_else(|| AppError::not_found("share_link_not_found", "Share link not found."))?;

    let created_by: Uuid = row.get("created_by");
    let created_by_principal_id: Uuid = row.get("created_by_principal_id");
    let member_can_create_share_links: bool = row.get("member_can_create_share_links");

    Ok(ResourceContext {
        owns_resource: owns_resource(
            created_by,
            created_by_principal_id,
            actor_user_id,
            actor_principal_id,
        ),
        member_share_links_enabled: member_can_create_share_links,
        ..ResourceContext::default()
    })
}

fn owns_resource(
    created_by: Uuid,
    created_by_principal_id: Uuid,
    actor_user_id: Uuid,
    actor_principal_id: Uuid,
) -> bool {
    created_by == actor_user_id || created_by_principal_id == actor_principal_id
}

pub async fn target_role_for_member(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    member_id: Uuid,
) -> Result<WorkspaceRole, AppError> {
    let row = sqlx::query(
        r#"
        SELECT role::text AS role
        FROM workspace_memberships
        WHERE workspace_id = $1 AND user_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| AppError::not_found("member_not_found", "Member not found."))?;
    parse_role(row.get::<String, _>("role").as_str())
}

fn validate_workspace_context(
    auth: &AuthContext,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    if auth
        .tenant_id
        .is_some_and(|tenant| tenant_id.is_some_and(|workspace_tenant| workspace_tenant != tenant))
    {
        return Err(AppError::forbidden(
            "workspace_access_denied",
            "You do not have access to this workspace.",
        ));
    }

    if auth.organization_id.is_some_and(|organization| {
        organization_id.is_some_and(|workspace_org| workspace_org != organization)
    }) {
        return Err(AppError::forbidden(
            "workspace_access_denied",
            "You do not have access to this workspace.",
        ));
    }

    if auth
        .workspace_id
        .is_some_and(|current_workspace| current_workspace != workspace_id)
    {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "Switch to this workspace before performing the requested action.",
        ));
    }

    Ok(())
}

fn validate_actor_context(
    auth: &AuthContext,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let Some(actor) = auth.actor.as_ref() else {
        return Ok(());
    };

    if actor.tenant_id.is_some_and(|actor_tenant_id| {
        tenant_id.is_some_and(|workspace_tenant| workspace_tenant != actor_tenant_id)
    }) {
        return Err(AppError::forbidden(
            "workspace_access_denied",
            "The delegated actor does not have access to this workspace.",
        ));
    }

    if actor.organization_id.is_some_and(|actor_org_id| {
        organization_id.is_some_and(|workspace_org| workspace_org != actor_org_id)
    }) {
        return Err(AppError::forbidden(
            "workspace_access_denied",
            "The delegated actor does not have access to this workspace.",
        ));
    }

    if actor
        .workspace_id
        .is_some_and(|actor_workspace_id| actor_workspace_id != workspace_id)
    {
        return Err(AppError::forbidden(
            "workspace_context_mismatch",
            "The delegated actor does not match this workspace.",
        ));
    }

    Ok(())
}

fn narrow_role(left: WorkspaceRole, right: WorkspaceRole) -> WorkspaceRole {
    if role_rank(left) >= role_rank(right) {
        right
    } else {
        left
    }
}

fn role_rank(role: WorkspaceRole) -> u8 {
    match role {
        WorkspaceRole::Owner => 3,
        WorkspaceRole::Admin => 2,
        WorkspaceRole::Member => 1,
        WorkspaceRole::Viewer => 0,
    }
}

#[cfg(test)]
#[path = "drive.domains.authz.db.tests.rs"]
mod tests;
