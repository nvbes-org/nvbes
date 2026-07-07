use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{service_status::sql_status, user_access::AccessScope};

pub async fn lock_tenant_owner_changes(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
) -> Result<(), Status> {
    sqlx::query("SELECT id FROM tenants WHERE id = $1 FOR UPDATE")
        .bind(tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(sql_status)?;
    Ok(())
}

pub async fn ensure_workspaces_belong(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_ids: &[Uuid],
    scope: AccessScope,
) -> Result<(), Status> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspaces
        WHERE tenant_id = $1
          AND id = ANY($2)
          AND ($3::uuid IS NULL OR organization_id = $3)
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_ids)
    .bind(scope.organization_id())
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    if count != workspace_ids.len() as i64 {
        return Err(Status::invalid_argument(
            "all workspace IDs must belong to the requested scope",
        ));
    }
    Ok(())
}

pub async fn target_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    scope: AccessScope,
) -> Result<Option<String>, Status> {
    role_query(tx, tenant_id, target_principal_id, scope, "active").await
}

pub async fn target_role_for_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    scope: AccessScope,
) -> Result<Option<String>, Status> {
    role_query(tx, tenant_id, target_principal_id, scope, "lifecycle").await
}

pub async fn target_workspace_ids_for_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    scope: AccessScope,
) -> Result<Vec<Uuid>, Status> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT wm.workspace_id
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE w.tenant_id = $1
          AND wm.principal_id = $2
          AND wm.status IN ('active', 'suspended')
          AND ($3::uuid IS NULL OR w.organization_id = $3)
        ORDER BY wm.workspace_id
        "#,
    )
    .bind(tenant_id)
    .bind(target_principal_id)
    .bind(scope.organization_id())
    .fetch_all(&mut **tx)
    .await
    .map_err(sql_status)
}

pub async fn replace_access(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    workspace_ids: &[Uuid],
    role: &str,
    scope: AccessScope,
) -> Result<(), Status> {
    remove_access_outside_selection(tx, tenant_id, target_principal_id, workspace_ids, scope)
        .await?;
    for workspace_id in workspace_ids {
        sqlx::query(
            r#"
            INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source)
            SELECT w.id, $2, $3::workspace_member_role, 'active', 'manual'
            FROM workspaces w
            WHERE w.id = $1 AND w.tenant_id = $4
            ON CONFLICT (workspace_id, principal_id)
            DO UPDATE SET role = EXCLUDED.role, status = 'active', source = 'manual', updated_at = NOW()
            "#,
        )
        .bind(workspace_id)
        .bind(target_principal_id)
        .bind(role)
        .bind(tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(sql_status)?;
    }
    Ok(())
}

pub async fn set_workspace_memberships_status(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    status: &str,
    workspace_ids: Option<&[Uuid]>,
    scope: AccessScope,
) -> Result<u64, Status> {
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
    .bind(target_principal_id)
    .bind(status)
    .bind(workspace_ids)
    .bind(scope.organization_id())
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(result.rows_affected())
}

pub async fn ensure_not_last_owner_after_access(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    workspace_ids: &[Uuid],
    role: &str,
    scope: AccessScope,
) -> Result<(), Status> {
    if role == "owner" {
        return Ok(());
    }
    let ownerless_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships current_owner
        INNER JOIN workspaces w ON w.id = current_owner.workspace_id
        WHERE w.tenant_id = $1
          AND current_owner.principal_id = $2
          AND current_owner.role = 'owner'
          AND current_owner.status = 'active'
          AND ($5::uuid IS NULL OR w.organization_id = $5)
          AND (
            NOT (current_owner.workspace_id = ANY($3))
            OR $4::workspace_member_role <> 'owner'
          )
          AND NOT EXISTS (
            SELECT 1
            FROM workspace_memberships other_owner
            WHERE other_owner.workspace_id = current_owner.workspace_id
              AND other_owner.principal_id <> $2
              AND other_owner.role = 'owner'
              AND other_owner.status = 'active'
          )
        "#,
    )
    .bind(tenant_id)
    .bind(target_principal_id)
    .bind(workspace_ids)
    .bind(role)
    .bind(scope.organization_id())
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    ensure_ownerless_count_zero(ownerless_count)
}

pub async fn ensure_not_last_owner_after_status(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    scope: AccessScope,
) -> Result<(), Status> {
    let ownerless_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships current_owner
        INNER JOIN workspaces w ON w.id = current_owner.workspace_id
        WHERE w.tenant_id = $1
          AND current_owner.principal_id = $2
          AND current_owner.role = 'owner'
          AND current_owner.status = 'active'
          AND ($3::uuid IS NULL OR w.organization_id = $3)
          AND NOT EXISTS (
            SELECT 1
            FROM workspace_memberships other_owner
            WHERE other_owner.workspace_id = current_owner.workspace_id
              AND other_owner.principal_id <> $2
              AND other_owner.role = 'owner'
              AND other_owner.status = 'active'
          )
        "#,
    )
    .bind(tenant_id)
    .bind(target_principal_id)
    .bind(scope.organization_id())
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    ensure_ownerless_count_zero(ownerless_count)
}

async fn role_query(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    scope: AccessScope,
    mode: &str,
) -> Result<Option<String>, Status> {
    let statuses = if mode == "active" {
        "ARRAY['active']::workspace_member_status[]"
    } else {
        "ARRAY['active', 'suspended']::workspace_member_status[]"
    };
    let query = format!(
        r#"
        SELECT wm.role::text
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE w.tenant_id = $1
          AND wm.principal_id = $2
          AND wm.status = ANY({statuses})
          AND ($3::uuid IS NULL OR w.organization_id = $3)
        ORDER BY CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END
        LIMIT 1
        "#
    );
    sqlx::query_scalar::<_, String>(&query)
        .bind(tenant_id)
        .bind(target_principal_id)
        .bind(scope.organization_id())
        .fetch_optional(&mut **tx)
        .await
        .map_err(sql_status)
}

async fn remove_access_outside_selection(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    target_principal_id: Uuid,
    workspace_ids: &[Uuid],
    scope: AccessScope,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        UPDATE workspace_memberships wm
        SET status = 'removed', updated_at = NOW()
        FROM workspaces w
        WHERE w.id = wm.workspace_id
          AND w.tenant_id = $1
          AND wm.principal_id = $2
          AND NOT (wm.workspace_id = ANY($3))
          AND ($4::uuid IS NULL OR w.organization_id = $4)
        "#,
    )
    .bind(tenant_id)
    .bind(target_principal_id)
    .bind(workspace_ids)
    .bind(scope.organization_id())
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

fn ensure_ownerless_count_zero(ownerless_count: i64) -> Result<(), Status> {
    if ownerless_count > 0 {
        Err(Status::failed_precondition(
            "every workspace must retain at least one active owner",
        ))
    } else {
        Ok(())
    }
}
