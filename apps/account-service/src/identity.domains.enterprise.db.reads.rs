use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    domains::{authz::AdminScope, cloud::workspace_port},
    http::error::AppError,
};

use super::records::{
    ActorAccessRow, EnterpriseInvitationRow, EnterpriseUserRow, WorkspaceSummaryRow,
};

pub async fn actor_access(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<Option<ActorAccessRow>, AppError> {
    let role = scoped_member_role(tenant_id, actor_id, actor_id, AdminScope::Tenant).await?;
    Ok(role.map(|role| ActorAccessRow {
        role,
        break_glass: false,
        break_glass_procedure_reference: None,
        break_glass_reason: None,
        break_glass_created_at: None,
        break_glass_last_used_at: None,
    }))
}

pub async fn list_users(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<Vec<EnterpriseUserRow>, AppError> {
    let workspace_access =
        workspace_access_by_principal(tenant_id, scope, actor_principal_id).await?;
    let users = user_bases(db, tenant_id, scope).await?;

    Ok(users
        .into_iter()
        .map(|user| {
            let access = workspace_access.get(&user.id);
            EnterpriseUserRow {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                role: access
                    .and_then(|access| access.role.clone())
                    .unwrap_or_else(|| "viewer".to_string()),
                break_glass_procedure_reference: None,
                break_glass_reason: None,
                break_glass_created_at: None,
                break_glass_last_used_at: None,
                workspace_ids: access.map(MemberAccess::workspace_ids).unwrap_or_default(),
                status: access
                    .map(MemberAccess::status)
                    .unwrap_or(user.tenant_status),
                mfa_enabled: user.mfa_enabled,
                last_seen_at: user.last_seen_at,
                created_at: user.created_at,
            }
        })
        .collect())
}

pub async fn list_invitations(
    _db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<Vec<EnterpriseInvitationRow>, AppError> {
    let mut rows = Vec::new();
    for workspace in scoped_workspaces(tenant_id, scope, actor_principal_id).await? {
        for invitation in workspace_port::list_workspace_invitations(
            Some(tenant_id),
            workspace.workspace_id,
            actor_principal_id,
            &["pending", "accepted"],
        )
        .await?
        {
            rows.push(EnterpriseInvitationRow {
                id: invitation.id,
                email: invitation.email,
                role: invitation.role,
                workspace_ids: vec![workspace.workspace_id],
                status: invitation.status,
                invited_at: invitation.created_at,
                expires_at: Some(invitation.expires_at),
            });
        }
    }
    rows.sort_by(|left, right| right.invited_at.cmp(&left.invited_at));
    Ok(rows)
}

pub async fn list_workspaces(
    _db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<Vec<WorkspaceSummaryRow>, AppError> {
    let mut rows = Vec::new();
    for workspace in scoped_workspaces(tenant_id, scope, actor_principal_id).await? {
        let member_count = workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            actor_principal_id,
        )
        .await?
        .into_iter()
        .filter(|member| member.active)
        .count() as i64;
        rows.push(WorkspaceSummaryRow {
            id: workspace.workspace_id,
            name: workspace.name,
            workspace_type: workspace.workspace_type,
            data_region: workspace.data_region,
            member_count,
            storage_used_bytes: 0,
            created_at: workspace.created_at,
        });
    }
    rows.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(rows)
}

pub async fn usage_metrics(
    _db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<(i64, i64, i64), AppError> {
    let workspaces = scoped_workspaces(tenant_id, scope, actor_principal_id).await?;
    let mut active_principals = HashSet::new();
    for workspace in &workspaces {
        for member in workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            actor_principal_id,
        )
        .await?
        {
            if member.active {
                active_principals.insert(member.principal_id);
            }
        }
    }
    Ok((workspaces.len() as i64, active_principals.len() as i64, 0))
}

async fn scoped_member_role(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    member_principal_id: Uuid,
    scope: AdminScope,
) -> Result<Option<String>, AppError> {
    let access = workspace_access_by_principal(tenant_id, scope, actor_principal_id).await?;
    Ok(access
        .get(&member_principal_id)
        .and_then(|member| member.role.clone()))
}

async fn workspace_access_by_principal(
    tenant_id: Uuid,
    scope: AdminScope,
    actor_principal_id: Uuid,
) -> Result<HashMap<Uuid, MemberAccess>, AppError> {
    let mut access_by_principal = HashMap::<Uuid, MemberAccess>::new();
    for workspace in scoped_workspaces(tenant_id, scope, actor_principal_id).await? {
        for member in workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            actor_principal_id,
        )
        .await?
        {
            if member.status != "active" && member.status != "suspended" {
                continue;
            }
            let access = access_by_principal.entry(member.principal_id).or_default();
            access.workspace_ids.insert(workspace.workspace_id);
            access.has_active |= member.status == "active";
            access.has_suspended |= member.status == "suspended";
            if role_rank(&member.role) < access.role.as_deref().map(role_rank).unwrap_or(4) {
                access.role = Some(member.role);
            }
        }
    }
    Ok(access_by_principal)
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

async fn user_bases(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<EnterpriseUserBaseRow>, AppError> {
    match scope {
        AdminScope::Tenant => Ok(sqlx::query_as::<_, EnterpriseUserBaseRow>(
            r#"
            SELECT
              u.principal_id AS id,
              u.email,
              COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS display_name,
              tm.status::text AS tenant_status,
              EXISTS (
                SELECT 1 FROM mfa_factors mf
                WHERE mf.principal_id = u.principal_id AND mf.status = 'active'
              ) AS mfa_enabled,
              NULL::timestamptz AS last_seen_at,
              u.created_at
            FROM tenant_memberships tm
            INNER JOIN users u ON u.principal_id = tm.principal_id
            WHERE tm.tenant_id = $1
            ORDER BY u.email ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(db)
        .await?),
        AdminScope::Organization(organization_id) => Ok(sqlx::query_as::<_, EnterpriseUserBaseRow>(
            r#"
            SELECT
              u.principal_id AS id,
              u.email,
              COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS display_name,
              tm.status::text AS tenant_status,
              EXISTS (
                SELECT 1 FROM mfa_factors mf
                WHERE mf.principal_id = u.principal_id AND mf.status = 'active'
              ) AS mfa_enabled,
              NULL::timestamptz AS last_seen_at,
              u.created_at
            FROM tenant_memberships tm
            INNER JOIN users u ON u.principal_id = tm.principal_id
            INNER JOIN organization_memberships om ON om.principal_id = tm.principal_id
              AND om.organization_id = $2
            WHERE tm.tenant_id = $1
            ORDER BY u.email ASC
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .fetch_all(db)
        .await?),
    }
}

#[derive(Debug, FromRow)]
struct EnterpriseUserBaseRow {
    id: Uuid,
    email: String,
    display_name: String,
    tenant_status: String,
    mfa_enabled: bool,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Default)]
struct MemberAccess {
    role: Option<String>,
    workspace_ids: HashSet<Uuid>,
    has_active: bool,
    has_suspended: bool,
}

impl MemberAccess {
    fn workspace_ids(&self) -> Vec<Uuid> {
        let mut workspace_ids = self.workspace_ids.iter().copied().collect::<Vec<_>>();
        workspace_ids.sort();
        workspace_ids
    }

    fn status(&self) -> String {
        if self.has_active {
            "active".to_string()
        } else if self.has_suspended {
            "suspended".to_string()
        } else {
            "removed".to_string()
        }
    }
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
