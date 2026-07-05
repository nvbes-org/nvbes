use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domains::authz::AdminScope;
use crate::http::error::AppError;

pub async fn target_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
) -> Result<Option<String>, AppError> {
    let role = match scope {
        AdminScope::Tenant => {
            sqlx::query_scalar::<_, String>(r#"
                SELECT wm.role::text
                FROM workspace_memberships wm
                INNER JOIN workspaces w ON w.id = wm.workspace_id
                WHERE w.tenant_id = $1 AND wm.principal_id = $2 AND wm.status = 'active'
                ORDER BY CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END
                LIMIT 1
                "#)
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await?
        }
        AdminScope::Organization(org_id) => {
            sqlx::query_scalar::<_, String>(r#"
                SELECT wm.role::text
                FROM workspace_memberships wm
                INNER JOIN workspaces w ON w.id = wm.workspace_id
                WHERE w.tenant_id = $1
                  AND w.organization_id = $3
                  AND wm.principal_id = $2
                  AND wm.status = 'active'
                ORDER BY CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END
                LIMIT 1
                "#)
            .bind(tenant_id)
            .bind(user_id)
            .bind(org_id)
            .fetch_optional(&mut **tx)
            .await?
        }
    };
    Ok(role)
}

pub async fn target_role_for_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
) -> Result<Option<String>, AppError> {
    let role = match scope {
        AdminScope::Tenant => {
            sqlx::query_scalar::<_, String>(r#"
                SELECT wm.role::text
                FROM workspace_memberships wm
                INNER JOIN workspaces w ON w.id = wm.workspace_id
                WHERE w.tenant_id = $1
                  AND wm.principal_id = $2
                  AND wm.status IN ('active', 'suspended')
                ORDER BY CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END
                LIMIT 1
                "#)
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await?
        }
        AdminScope::Organization(org_id) => {
            sqlx::query_scalar::<_, String>(r#"
                SELECT wm.role::text
                FROM workspace_memberships wm
                INNER JOIN workspaces w ON w.id = wm.workspace_id
                WHERE w.tenant_id = $1
                  AND w.organization_id = $3
                  AND wm.principal_id = $2
                  AND wm.status IN ('active', 'suspended')
                ORDER BY CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END
                LIMIT 1
                "#)
            .bind(tenant_id)
            .bind(user_id)
            .bind(org_id)
            .fetch_optional(&mut **tx)
            .await?
        }
    };
    Ok(role)
}

pub async fn target_workspace_ids_for_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<Uuid>, AppError> {
    let workspace_ids = match scope {
        AdminScope::Tenant => {
            sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT wm.workspace_id
                FROM workspace_memberships wm
                INNER JOIN workspaces w ON w.id = wm.workspace_id
                WHERE w.tenant_id = $1
                  AND wm.principal_id = $2
                  AND wm.status IN ('active', 'suspended')
                ORDER BY wm.workspace_id
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_all(&mut **tx)
            .await?
        }
        AdminScope::Organization(org_id) => {
            sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT wm.workspace_id
                FROM workspace_memberships wm
                INNER JOIN workspaces w ON w.id = wm.workspace_id
                WHERE w.tenant_id = $1
                  AND w.organization_id = $3
                  AND wm.principal_id = $2
                  AND wm.status IN ('active', 'suspended')
                ORDER BY wm.workspace_id
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(org_id)
            .fetch_all(&mut **tx)
            .await?
        }
    };
    Ok(workspace_ids)
}

pub async fn replace_access(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    workspace_ids: &[Uuid],
    role: &str,
    scope: AdminScope,
) -> Result<(), AppError> {
    remove_access_outside_selection(tx, tenant_id, user_id, workspace_ids, scope).await?;
    for workspace_id in workspace_ids {
        sqlx::query(r#"
            INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source)
            SELECT w.id, $2, $3::workspace_member_role, 'active', 'manual'
            FROM workspaces w
            WHERE w.id = $1 AND w.tenant_id = $4
            ON CONFLICT (workspace_id, principal_id)
            DO UPDATE SET role = EXCLUDED.role, status = 'active', source = 'manual', updated_at = NOW()
            "#)
        .bind(workspace_id)
        .bind(user_id)
        .bind(role)
        .bind(tenant_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn remove_access_outside_selection(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    workspace_ids: &[Uuid],
    scope: AdminScope,
) -> Result<(), AppError> {
    match scope {
        AdminScope::Tenant => {
            sqlx::query(
                r#"
                UPDATE workspace_memberships wm
                SET status = 'removed', updated_at = NOW()
                FROM workspaces w
                WHERE w.id = wm.workspace_id
                  AND w.tenant_id = $1
                  AND wm.principal_id = $2
                  AND NOT (wm.workspace_id = ANY($3))
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(workspace_ids)
            .execute(&mut **tx)
            .await?;
        }
        AdminScope::Organization(org_id) => {
            sqlx::query(
                r#"
                UPDATE workspace_memberships wm
                SET status = 'removed', updated_at = NOW()
                FROM workspaces w
                WHERE w.id = wm.workspace_id
                  AND w.tenant_id = $1
                  AND w.organization_id = $4
                  AND wm.principal_id = $2
                  AND NOT (wm.workspace_id = ANY($3))
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(workspace_ids)
            .bind(org_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

pub async fn set_tenant_memberships_status(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    status: &str,
    workspace_ids: Option<&[Uuid]>,
    scope: AdminScope,
) -> Result<u64, AppError> {
    let result = match (workspace_ids, scope) {
        (Some(workspace_ids), AdminScope::Tenant) => {
            update_status(tx, tenant_id, user_id, status, Some(workspace_ids), None).await?
        }
        (Some(workspace_ids), AdminScope::Organization(org_id)) => {
            update_status(
                tx,
                tenant_id,
                user_id,
                status,
                Some(workspace_ids),
                Some(org_id),
            )
            .await?
        }
        (None, AdminScope::Tenant) => {
            update_status(tx, tenant_id, user_id, status, None, None).await?
        }
        (None, AdminScope::Organization(org_id)) => {
            update_status(tx, tenant_id, user_id, status, None, Some(org_id)).await?
        }
    };
    Ok(result)
}

async fn update_status(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    status: &str,
    workspace_ids: Option<&[Uuid]>,
    organization_id: Option<Uuid>,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE workspace_memberships wm
        SET status = $3::workspace_member_status, updated_at = NOW()
        FROM workspaces w
        WHERE w.id = wm.workspace_id
          AND w.tenant_id = $1
          AND wm.principal_id = $2
          AND ($4::uuid[] IS NULL OR wm.workspace_id = ANY($4))
          AND ($5::uuid IS NULL OR w.organization_id = $5)
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(status)
    .bind(workspace_ids)
    .bind(organization_id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
