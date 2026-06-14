use chrono::{Duration, Utc};
use nvbes_audit::AuditEventInput;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::records::EnterpriseInvitationRow;
use crate::http::error::AppError;

pub async fn ensure_workspaces_belong(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_ids: &[Uuid],
) -> Result<(), AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1 AND id = ANY($2)",
    )
    .bind(tenant_id)
    .bind(workspace_ids)
    .fetch_one(&mut **tx)
    .await?;
    if count != workspace_ids.len() as i64 {
        return Err(AppError::bad_request(
            "invalid_workspace_scope",
            "All workspace IDs must belong to the active tenant.",
        ));
    }
    Ok(())
}

pub async fn target_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<Option<String>, AppError> {
    let role = sqlx::query_scalar::<_, String>(r#"
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
    .await?;
    Ok(role)
}

pub async fn replace_access(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    workspace_ids: &[Uuid],
    role: &str,
) -> Result<(), AppError> {
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

pub async fn set_tenant_memberships_status(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
    status: &str,
    workspace_ids: Option<&[Uuid]>,
) -> Result<u64, AppError> {
    let result = if let Some(workspace_ids) = workspace_ids {
        sqlx::query(
            r#"
            UPDATE workspace_memberships wm
            SET status = $3::workspace_member_status, updated_at = NOW()
            FROM workspaces w
            WHERE w.id = wm.workspace_id
              AND w.tenant_id = $1
              AND wm.principal_id = $2
              AND wm.workspace_id = ANY($4)
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(status)
        .bind(workspace_ids)
        .execute(&mut **tx)
        .await?
    } else {
        sqlx::query(
            r#"
            UPDATE workspace_memberships wm
            SET status = $3::workspace_member_status, updated_at = NOW()
            FROM workspaces w
            WHERE w.id = wm.workspace_id AND w.tenant_id = $1 AND wm.principal_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(status)
        .execute(&mut **tx)
        .await?
    };
    Ok(result.rows_affected())
}

pub async fn insert_invitation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    email: &str,
    role: &str,
    token_hash: &str,
) -> Result<EnterpriseInvitationRow, AppError> {
    let expires_at = Utc::now() + Duration::days(7);
    Ok(sqlx::query_as::<_, EnterpriseInvitationRow>(
        r#"
        INSERT INTO workspace_invitations (workspace_id, email, role, token_hash, expires_at)
        SELECT w.id, $3, $4::workspace_member_role, $5, $6
        FROM workspaces w
        WHERE w.id = $1 AND w.tenant_id = $2
        RETURNING id, email, role::text AS role, ARRAY[workspace_id] AS workspace_ids,
          status::text AS status, created_at AS invited_at, expires_at
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(email)
    .bind(role)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn has_pending_invitation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    email: &str,
) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_invitations wi
        INNER JOIN workspaces w ON w.id = wi.workspace_id
        WHERE w.tenant_id = $1
          AND wi.workspace_id = $2
          AND lower(wi.email) = $3
          AND wi.status = 'pending'
          AND wi.expires_at > NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(email)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count > 0)
}

pub async fn insert_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
    target_type: &'static str,
    target_id: Option<Uuid>,
    metadata: Value,
) -> Result<(), AppError> {
    nvbes_audit::insert_audit_event_tx(
        &mut **tx,
        AuditEventInput {
            tenant_id,
            workspace_id: None,
            actor_principal_id: Some(actor_id),
            action,
            target_type,
            target_id,
            ip: None,
            user_agent: None,
            metadata,
        },
    )
    .await?;
    Ok(())
}
