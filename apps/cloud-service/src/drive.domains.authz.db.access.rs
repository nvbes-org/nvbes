use sqlx::Row;
use uuid::Uuid;

use crate::{
    domains::{
        auth::types::{AuthContext, AuthPrincipalKind},
        authz::types::{WorkspaceAccess, WorkspacePolicy},
    },
    http::error::AppError,
};
use nvbes_tenancy::workspace::WorkspaceAccessError;

use super::role_policy::{narrow_role, parse_token_role};
use super::workspace_scope::{validate_actor_context, validate_workspace_context};

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
            WorkspaceAccessError::ContextMismatch.to_string(),
        ));
    }

    if auth.principal_kind == AuthPrincipalKind::ServiceAccount || auth.role.is_some() {
        return load_token_workspace_access(db, auth, workspace_id).await;
    }

    load_membership_workspace_access(db, auth, workspace_id).await
}

async fn load_token_workspace_access(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    workspace_id: Uuid,
) -> Result<WorkspaceAccess, AppError> {
    let token_role = auth.role.as_deref().ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            WorkspaceAccessError::PrincipalRoleMissing.to_string(),
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
            WorkspaceAccessError::AccessDenied.to_string(),
        )
    })?;

    let tenant_id: Option<Uuid> = row.get("tenant_id");
    let organization_id: Option<Uuid> = row.get("organization_id");
    validate_workspace_context(auth, tenant_id, organization_id, workspace_id)?;
    validate_actor_context(auth, tenant_id, organization_id, workspace_id)?;

    let parsed_role = parse_token_role(token_role)?;
    let effective_role = auth
        .actor
        .as_ref()
        .and_then(|actor| actor.role.as_deref())
        .map(parse_token_role)
        .transpose()?
        .map(|actor_role| narrow_role(parsed_role, actor_role))
        .unwrap_or(parsed_role);

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

async fn load_membership_workspace_access(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    workspace_id: Uuid,
) -> Result<WorkspaceAccess, AppError> {
    let row = sqlx::query(
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
    .await?;

    let row = row.ok_or_else(|| {
        AppError::forbidden(
            "workspace_access_denied",
            WorkspaceAccessError::AccessDenied.to_string(),
        )
    })?;

    let tenant_id: Option<Uuid> = row.get("tenant_id");
    let organization_id: Option<Uuid> = row.get("organization_id");
    validate_workspace_context(auth, tenant_id, organization_id, workspace_id)?;
    validate_actor_context(auth, tenant_id, organization_id, workspace_id)?;

    let user_role = parse_token_role(row.get::<String, _>("role").as_str())?;
    let effective_role = auth
        .actor
        .as_ref()
        .and_then(|actor| actor.role.as_deref())
        .map(parse_token_role)
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
