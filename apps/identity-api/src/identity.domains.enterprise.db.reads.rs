use sqlx::PgPool;
use uuid::Uuid;

use super::records::{
    ActorAccessRow, AuditEventRow, BillingSummaryRow, DeveloperCredentialRow,
    EnterpriseInvitationRow, EnterpriseUserRow, InvoiceRow, PolicySummaryRow, SecuritySummaryRow,
    WorkspaceSummaryRow,
};
use crate::http::error::AppError;

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
        LEFT JOIN (
          workspace_memberships wm
          INNER JOIN workspaces w ON w.id = wm.workspace_id AND w.tenant_id = $1
        ) ON wm.principal_id = u.principal_id AND wm.status IN ('active', 'suspended')
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
          COALESCE(s.status::text, 'inactive') AS status, NULL::text AS billing_email
        FROM workspaces w
        LEFT JOIN subscriptions s ON s.workspace_id = w.id
        LEFT JOIN plans p ON p.code = w.plan_code OR p.id = s.plan_id
        LEFT JOIN billing_accounts ba ON ba.workspace_id = w.id
        WHERE w.tenant_id = $1
        ORDER BY CASE COALESCE(s.status::text, 'inactive') WHEN 'active' THEN 0 ELSE 1 END, w.created_at DESC
        LIMIT 1
        "#)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?)
}

pub async fn list_developers(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperCredentialRow>, AppError> {
    Ok(sqlx::query_as::<_, DeveloperCredentialRow>(
        r#"
        SELECT oc.id, oc.name, NULL::text AS owner_email,
          COALESCE(policy.scopes, ARRAY[]::text[]) AS scopes,
          oc.created_at
        FROM oauth_clients oc
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope) AS scopes
          FROM oauth_client_policies ocp
          CROSS JOIN LATERAL unnest(ocp.allowed_scopes) AS scope(scope)
          WHERE ocp.client_id = oc.id AND ocp.status = 'active'
        ) policy ON true
        WHERE oc.tenant_id = $1 AND oc.revoked_at IS NULL
        ORDER BY oc.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn list_policies(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<PolicySummaryRow>, AppError> {
    Ok(sqlx::query_as::<_, PolicySummaryRow>(
        r#"
        SELECT wp.workspace_id AS id,
          CONCAT('Workspace policy: ', w.name) AS name,
          'workspace'::text AS category,
          true AS enabled,
          jsonb_build_object(
            'member_can_create_share_links', wp.member_can_create_share_links,
            'require_admin_approval_for_member_share', wp.require_admin_approval_for_member_share,
            'default_share_link_ttl_days', wp.default_share_link_ttl_days,
            'max_share_link_ttl_days', wp.max_share_link_ttl_days,
            'required_acr', wp.required_acr::text
          ) AS configuration,
          wp.updated_at
        FROM workspace_policies wp
        INNER JOIN workspaces w ON w.id = wp.workspace_id
        WHERE w.tenant_id = $1
        ORDER BY wp.updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn security_summary(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<SecuritySummaryRow, AppError> {
    Ok(sqlx::query_as::<_, SecuritySummaryRow>(
        r#"
        SELECT
          COUNT(DISTINCT mf.id) FILTER (WHERE mf.status = 'active')::bigint AS mfa_factor_count,
          COUNT(DISTINCT mf.id) FILTER (WHERE mf.status = 'active' AND mf.factor_type = 'webauthn')::bigint AS passkey_count,
          COUNT(DISTINCT re.id) FILTER (WHERE re.risk_score >= 0.7)::bigint AS high_risk_event_count,
          COUNT(DISTINCT tm.principal_id) FILTER (
            WHERE tm.status = 'active'
              AND EXISTS (
                SELECT 1
                FROM workspace_memberships admin_wm
                INNER JOIN workspaces admin_w ON admin_w.id = admin_wm.workspace_id
                WHERE admin_w.tenant_id = tm.tenant_id
                  AND admin_wm.principal_id = tm.principal_id
                  AND admin_wm.status = 'active'
                  AND admin_wm.role IN ('owner', 'admin')
              )
              AND NOT EXISTS (
                SELECT 1 FROM mfa_factors admin_mf
                WHERE admin_mf.principal_id = tm.principal_id
                  AND admin_mf.status = 'active'
              )
          )::bigint AS admin_without_mfa_count,
          (
            SELECT COUNT(*)::bigint
            FROM tenant_domains td
            WHERE td.tenant_id = $1 AND td.verified_at IS NOT NULL
          ) AS verified_domain_count,
          (
            SELECT COUNT(*)::bigint
            FROM federated_identity_providers fip
            WHERE fip.tenant_id = $1 AND fip.status = 'active'
          ) AS sso_provider_count,
          (
            SELECT COUNT(*)::bigint
            FROM developer_client_secret_versions dcsv
            WHERE dcsv.tenant_id = $1
              AND dcsv.revoked_at IS NULL
              AND (
                (dcsv.status = 'overlap' AND dcsv.expires_at <= NOW())
                OR (dcsv.status = 'active' AND dcsv.created_at <= NOW() - INTERVAL '90 days')
              )
          ) AS stale_secret_count
        FROM tenant_memberships tm
        LEFT JOIN mfa_factors mf ON mf.principal_id = tm.principal_id
        LEFT JOIN risk_events re ON re.principal_id = tm.principal_id
        WHERE tm.tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?)
}

pub async fn list_invoices(db: &PgPool, tenant_id: Uuid) -> Result<Vec<InvoiceRow>, AppError> {
    Ok(sqlx::query_as::<_, InvoiceRow>(
        r#"
        SELECT ie.id, 'estimated'::text AS status,
          ie.estimated_amount_cents AS amount_due_cents,
          ie.created_at AS issued_at
        FROM invoice_estimates ie
        INNER JOIN workspaces w ON w.id = ie.workspace_id
        WHERE w.tenant_id = $1
        ORDER BY ie.created_at DESC
        LIMIT 12
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
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
