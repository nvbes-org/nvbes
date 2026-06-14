use chrono::{DateTime, Duration, Utc};
use nvbes_audit::AuditEventInput;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{policy, types::*};
use crate::http::error::AppError;

#[derive(Debug, FromRow)]
pub struct ActorAccessRow {
    pub role: String,
}

#[derive(Debug, FromRow)]
pub struct EnterpriseUserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub mfa_enabled: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl EnterpriseUserRow {
    pub fn into_view(self) -> EnterpriseUser {
        let role = policy::role_from_db(&self.role);
        EnterpriseUser {
            module_grants: policy::grants_for_role(&role),
            role,
            id: self.id,
            email: self.email,
            display_name: self.display_name,
            workspace_ids: self.workspace_ids,
            status: self.status,
            mfa_enabled: self.mfa_enabled,
            last_seen_at: self.last_seen_at,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct EnterpriseInvitationRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub workspace_ids: Vec<Uuid>,
    pub status: String,
    pub invited_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl EnterpriseInvitationRow {
    pub fn into_view(self) -> EnterpriseInvitation {
        let role = policy::role_from_db(&self.role);
        EnterpriseInvitation {
            module_grants: policy::grants_for_role(&role),
            role,
            id: self.id,
            email: self.email,
            workspace_ids: self.workspace_ids,
            status: self.status,
            invited_at: self.invited_at,
            expires_at: self.expires_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct WorkspaceSummaryRow {
    pub id: Uuid,
    pub name: String,
    pub workspace_type: String,
    pub data_region: Option<String>,
    pub member_count: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
}

impl WorkspaceSummaryRow {
    pub fn into_view(self) -> EnterpriseWorkspaceSummary {
        EnterpriseWorkspaceSummary {
            id: self.id,
            name: self.name,
            workspace_type: self.workspace_type,
            data_region: self.data_region,
            member_count: self.member_count,
            storage_used_bytes: self.storage_used_bytes,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct AuditEventRow {
    pub id: Uuid,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

impl AuditEventRow {
    pub fn into_view(self) -> EnterpriseAuditEvent {
        EnterpriseAuditEvent {
            id: self.id,
            event_type: self.event_type,
            actor_id: self.actor_id,
            actor_email: self.actor_email,
            target_type: self.target_type,
            target_id: self.target_id,
            metadata: serde_json::from_value(self.metadata).ok(),
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct BillingSummaryRow {
    pub code: String,
    pub name: String,
    pub status: String,
    pub billing_email: Option<String>,
}

pub async fn actor_access(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<Option<ActorAccessRow>, AppError> {
    Ok(sqlx::query_as::<_, ActorAccessRow>(
        r#"
        SELECT wm.role::text AS role
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        WHERE w.tenant_id = $1 AND wm.principal_id = $2 AND wm.status = 'active'
        ORDER BY CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_optional(db)
    .await?)
}

pub async fn list_users(db: &PgPool, tenant_id: Uuid) -> Result<Vec<EnterpriseUserRow>, AppError> {
    Ok(sqlx::query_as::<_, EnterpriseUserRow>(r#"
        SELECT
          u.principal_id AS id,
          u.email,
          COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS display_name,
          CASE MIN(CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 WHEN 'viewer' THEN 3 ELSE 4 END)
            WHEN 0 THEN 'owner'
            WHEN 1 THEN 'admin'
            WHEN 2 THEN 'member'
            ELSE 'viewer'
          END AS role,
          COALESCE(array_agg(DISTINCT w.id) FILTER (WHERE w.id IS NOT NULL), ARRAY[]::uuid[]) AS workspace_ids,
          CASE WHEN bool_or(wm.status = 'active') THEN 'active'
               WHEN bool_or(wm.status = 'suspended') THEN 'suspended'
               ELSE tm.status::text END AS status,
          EXISTS (
            SELECT 1 FROM mfa_factors mf
            WHERE mf.principal_id = u.principal_id AND mf.status = 'active'
          ) AS mfa_enabled,
          NULL::timestamptz AS last_seen_at,
          u.created_at
        FROM tenant_memberships tm
        INNER JOIN users u ON u.principal_id = tm.principal_id
        LEFT JOIN workspace_memberships wm ON wm.principal_id = u.principal_id
          AND wm.status IN ('active', 'suspended')
        LEFT JOIN workspaces w ON w.id = wm.workspace_id AND w.tenant_id = tm.tenant_id
        WHERE tm.tenant_id = $1
        GROUP BY u.principal_id, u.email, u.firstname, u.lastname, u.username, u.created_at, tm.status
        ORDER BY u.email ASC
        "#)
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn list_invitations(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<EnterpriseInvitationRow>, AppError> {
    Ok(sqlx::query_as::<_, EnterpriseInvitationRow>(
        r#"
        SELECT wi.id, wi.email, wi.role::text AS role, ARRAY[wi.workspace_id] AS workspace_ids,
          wi.status::text AS status, wi.created_at AS invited_at, wi.expires_at
        FROM workspace_invitations wi
        INNER JOIN workspaces w ON w.id = wi.workspace_id
        WHERE w.tenant_id = $1 AND wi.status IN ('pending', 'accepted')
        ORDER BY wi.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn list_workspaces(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<WorkspaceSummaryRow>, AppError> {
    Ok(sqlx::query_as::<_, WorkspaceSummaryRow>(
        r#"
        SELECT w.id, w.name, w.workspace_type::text AS workspace_type,
          w.data_region::text AS data_region,
          COUNT(wm.principal_id) FILTER (WHERE wm.status = 'active') AS member_count,
          COALESCE(qu.used_storage_bytes, 0)::bigint AS storage_used_bytes,
          w.created_at
        FROM workspaces w
        LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id
        LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
        WHERE w.tenant_id = $1
        GROUP BY w.id, qu.used_storage_bytes
        ORDER BY w.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    limit: i64,
) -> Result<Vec<AuditEventRow>, AppError> {
    Ok(sqlx::query_as::<_, AuditEventRow>(
        r#"
        SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
          u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
        WHERE ae.tenant_id = $1
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $2
        "#,
    )
    .bind(tenant_id)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

pub async fn billing_summary(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Option<BillingSummaryRow>, AppError> {
    Ok(sqlx::query_as::<_, BillingSummaryRow>(r#"
        SELECT COALESCE(p.code, w.plan_code) AS code, COALESCE(p.name, w.plan_code) AS name,
          COALESCE(s.status::text, 'inactive') AS status, ba.billing_email
        FROM workspaces w
        LEFT JOIN subscriptions s ON s.workspace_id = w.id
        LEFT JOIN plans p ON p.id = COALESCE(w.plan_id, s.plan_id)
        LEFT JOIN billing_accounts ba ON ba.workspace_id = w.id
        WHERE w.tenant_id = $1
        ORDER BY CASE COALESCE(s.status::text, 'inactive') WHEN 'active' THEN 0 ELSE 1 END, w.created_at DESC
        LIMIT 1
        "#)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?)
}

pub async fn usage_metrics(db: &PgPool, tenant_id: Uuid) -> Result<(i64, i64, i64), AppError> {
    Ok(sqlx::query_as::<_, (i64, i64, i64)>(
        r#"
        SELECT COUNT(DISTINCT w.id)::bigint, COUNT(DISTINCT wm.principal_id)::bigint,
          COALESCE(SUM(qu.used_storage_bytes), 0)::bigint
        FROM workspaces w
        LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id AND wm.status = 'active'
        LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
        WHERE w.tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?)
}

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
            VALUES ($1, $2, $3::workspace_member_role, 'active', 'manual')
            ON CONFLICT (workspace_id, principal_id)
            DO UPDATE SET role = EXCLUDED.role, status = 'active', source = 'manual', updated_at = NOW()
            "#)
        .bind(workspace_id)
        .bind(user_id)
        .bind(role)
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
) -> Result<(), AppError> {
    if let Some(workspace_ids) = workspace_ids {
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
        .await?;
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
        .await?;
    }
    Ok(())
}

pub async fn insert_invitation(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    email: &str,
    role: &str,
    token_hash: &str,
) -> Result<EnterpriseInvitationRow, AppError> {
    let expires_at = Utc::now() + Duration::days(7);
    Ok(sqlx::query_as::<_, EnterpriseInvitationRow>(
        r#"
        INSERT INTO workspace_invitations (workspace_id, email, role, token_hash, expires_at)
        VALUES ($1, $2, $3::workspace_member_role, $4, $5)
        RETURNING id, email, role::text AS role, ARRAY[workspace_id] AS workspace_ids,
          status::text AS status, created_at AS invited_at, expires_at
        "#,
    )
    .bind(workspace_id)
    .bind(email)
    .bind(role)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn has_pending_invitation(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    email: &str,
) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM workspace_invitations
        WHERE workspace_id = $1 AND lower(email) = $2 AND status = 'pending' AND expires_at > NOW()
        "#,
    )
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
