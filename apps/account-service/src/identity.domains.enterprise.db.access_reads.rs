use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domains::{authz::AdminScope, cloud::workspace_port},
    http::error::AppError,
};

pub async fn target_role(
    _tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<Option<String>, AppError> {
    scoped_member_role(tenant_id, user_id, scope, actor_principal_id, &["active"]).await
}

pub async fn target_role_for_lifecycle(
    _tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<Option<String>, AppError> {
    scoped_member_role(
        tenant_id,
        user_id,
        scope,
        actor_principal_id,
        &["active", "suspended"],
    )
    .await
}

async fn scoped_member_role(
    tenant_id: Uuid,
    principal_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
    statuses: &[&str],
) -> Result<Option<String>, AppError> {
    let mut best_role = None::<String>;
    for workspace in scoped_workspaces(tenant_id, scope, actor_principal_id).await? {
        for member in workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            actor_principal_id,
        )
        .await?
        {
            if member.principal_id != principal_id
                || !statuses.iter().any(|status| *status == member.status)
            {
                continue;
            }

            if role_rank(&member.role) < best_role.as_deref().map(role_rank).unwrap_or(4) {
                best_role = Some(member.role);
            }
        }
    }
    Ok(best_role)
}

async fn scoped_workspaces(
    tenant_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<Vec<workspace_port::CloudWorkspaceSummary>, AppError> {
    let organization_id = match scope {
        AdminScope::Tenant => None,
        AdminScope::Organization(organization_id) => Some(organization_id),
    };
    workspace_port::list_tenant_workspaces(tenant_id, organization_id, actor_principal_id).await
}

fn role_rank(role: &str) -> u8 {
    match role {
        "owner" => 0,
        "admin" => 1,
        "member" => 2,
        "viewer" => 3,
        _ => 4,
    }
}
