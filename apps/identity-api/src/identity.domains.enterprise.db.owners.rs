use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn active_owner_count(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT wm.principal_id)
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE w.tenant_id = $1 AND wm.role = 'owner' AND wm.status = 'active'
        "#,
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn lock_tenant_owner_changes(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text)::bigint)")
        .bind(tenant_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn ownerless_workspace_count_after_access(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    workspace_ids: &[Uuid],
    next_role: &str,
) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships current_owner
        INNER JOIN workspaces w ON w.id = current_owner.workspace_id
        WHERE w.tenant_id = $1
          AND current_owner.principal_id = $2
          AND current_owner.role = 'owner'
          AND current_owner.status = 'active'
          AND ($4 <> 'owner' OR NOT (current_owner.workspace_id = ANY($3)))
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
    .bind(user_id)
    .bind(workspace_ids)
    .bind(next_role)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn ownerless_workspace_count_after_status(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_memberships current_owner
        INNER JOIN workspaces w ON w.id = current_owner.workspace_id
        WHERE w.tenant_id = $1
          AND current_owner.principal_id = $2
          AND current_owner.role = 'owner'
          AND current_owner.status = 'active'
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
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?)
}
