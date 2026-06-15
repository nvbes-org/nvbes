use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::authz::AdminScope;

use super::records::{
    ActorAccessRow, AuditEventRow, EnterpriseInvitationRow, EnterpriseUserRow, WorkspaceSummaryRow,
};
use crate::http::error::AppError;

pub async fn actor_access(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<Option<ActorAccessRow>, AppError> {
    Ok(sqlx::query_as::<_, ActorAccessRow>(
        r#"
        SELECT wm.role::text AS role,
          tbga.principal_id IS NOT NULL AS break_glass,
          tbga.procedure_reference AS break_glass_procedure_reference,
          tbga.reason AS break_glass_reason,
          tbga.created_at AS break_glass_created_at,
          tbga.last_used_at AS break_glass_last_used_at
        FROM workspace_memberships wm
        INNER JOIN workspaces w ON w.id = wm.workspace_id
        LEFT JOIN tenant_break_glass_accounts tbga
          ON tbga.tenant_id = $1
         AND tbga.principal_id = $2
         AND tbga.revoked_at IS NULL
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

pub async fn list_users(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<EnterpriseUserRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
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
                  tbga.procedure_reference AS break_glass_procedure_reference,
                  tbga.reason AS break_glass_reason,
                  tbga.created_at AS break_glass_created_at,
                  tbga.last_used_at AS break_glass_last_used_at,
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
                LEFT JOIN (
                  workspace_memberships wm
                  INNER JOIN workspaces w ON w.id = wm.workspace_id AND w.tenant_id = $1
                ) ON wm.principal_id = u.principal_id AND wm.status IN ('active', 'suspended')
                LEFT JOIN tenant_break_glass_accounts tbga
                  ON tbga.tenant_id = tm.tenant_id
                 AND tbga.principal_id = tm.principal_id
                 AND tbga.revoked_at IS NULL
                WHERE tm.tenant_id = $1
                GROUP BY u.principal_id, u.email, u.firstname, u.lastname, u.username, u.created_at,
                  tm.status, tbga.procedure_reference, tbga.reason, tbga.created_at, tbga.last_used_at
                ORDER BY u.email ASC
                "#)
            .bind(tenant_id)
            .fetch_all(db)
            .await?)
        }
        AdminScope::Organization(org_id) => {
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
                  tbga.procedure_reference AS break_glass_procedure_reference,
                  tbga.reason AS break_glass_reason,
                  tbga.created_at AS break_glass_created_at,
                  tbga.last_used_at AS break_glass_last_used_at,
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
                INNER JOIN organization_memberships om ON om.principal_id = tm.principal_id AND om.organization_id = $2
                LEFT JOIN (
                  workspace_memberships wm
                  INNER JOIN workspaces w ON w.id = wm.workspace_id AND w.tenant_id = $1 AND w.organization_id = $2
                ) ON wm.principal_id = u.principal_id AND wm.status IN ('active', 'suspended')
                LEFT JOIN tenant_break_glass_accounts tbga
                  ON tbga.tenant_id = tm.tenant_id
                 AND tbga.principal_id = tm.principal_id
                 AND tbga.revoked_at IS NULL
                WHERE tm.tenant_id = $1
                GROUP BY u.principal_id, u.email, u.firstname, u.lastname, u.username, u.created_at,
                  tm.status, tbga.procedure_reference, tbga.reason, tbga.created_at, tbga.last_used_at
                ORDER BY u.email ASC
                "#)
            .bind(tenant_id)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}

pub async fn list_invitations(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<EnterpriseInvitationRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
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
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, EnterpriseInvitationRow>(
                r#"
                SELECT wi.id, wi.email, wi.role::text AS role, ARRAY[wi.workspace_id] AS workspace_ids,
                  wi.status::text AS status, wi.created_at AS invited_at, wi.expires_at
                FROM workspace_invitations wi
                INNER JOIN workspaces w ON w.id = wi.workspace_id
                WHERE w.tenant_id = $1 AND w.organization_id = $2 AND wi.status IN ('pending', 'accepted')
                ORDER BY wi.created_at DESC
                "#,
            )
            .bind(tenant_id)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}

pub async fn list_workspaces(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<WorkspaceSummaryRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
            Ok(sqlx::query_as::<_, WorkspaceSummaryRow>(
                r#"
                SELECT w.id, w.name, w.workspace_type::text AS workspace_type,
                  NULL::text AS data_region,
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
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, WorkspaceSummaryRow>(
                r#"
                SELECT w.id, w.name, w.workspace_type::text AS workspace_type,
                  NULL::text AS data_region,
                  COUNT(wm.principal_id) FILTER (WHERE wm.status = 'active') AS member_count,
                  COALESCE(qu.used_storage_bytes, 0)::bigint AS storage_used_bytes,
                  w.created_at
                FROM workspaces w
                LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id
                LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
                WHERE w.tenant_id = $1 AND w.organization_id = $2
                GROUP BY w.id, qu.used_storage_bytes
                ORDER BY w.created_at DESC
                "#,
            )
            .bind(tenant_id)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}

pub async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    limit: i64,
) -> Result<Vec<AuditEventRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
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
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, AuditEventRow>(
                r#"
                SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
                  u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
                FROM audit_events ae
                LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
                WHERE ae.tenant_id = $1
                  AND ae.workspace_id IN (SELECT id FROM workspaces WHERE organization_id = $3)
                ORDER BY ae.created_at DESC, ae.id DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}



pub async fn usage_metrics(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<(i64, i64, i64), AppError> {
    match scope {
        AdminScope::Tenant => {
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
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, (i64, i64, i64)>(
                r#"
                SELECT COUNT(DISTINCT w.id)::bigint, COUNT(DISTINCT wm.principal_id)::bigint,
                  COALESCE(SUM(qu.used_storage_bytes), 0)::bigint
                FROM workspaces w
                LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id AND wm.status = 'active'
                LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
                WHERE w.tenant_id = $1 AND w.organization_id = $2
                "#,
            )
            .bind(tenant_id)
            .bind(org_id)
            .fetch_one(db)
            .await?)
        }
    }
}
