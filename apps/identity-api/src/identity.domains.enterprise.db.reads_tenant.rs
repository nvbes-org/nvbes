use sqlx::PgPool;
use uuid::Uuid;

use super::records::{
    BillingSummaryRow, DeveloperCredentialRow, InvoiceRow, MfaPolicyRow, PolicySummaryRow,
    SecuritySummaryRow, SessionPolicyRow,
};
use crate::http::error::AppError;

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
        SELECT dcsv.id,
          dcsv.client_id,
          CONCAT(oc.name, ' secret ', dcsv.secret_last4) AS name,
          dcsv.status::text AS status,
          dcsv.secret_last4,
          NULL::text AS owner_email,
          COALESCE(policy.scopes, ARRAY[]::text[]) AS scopes,
          oc.last_used_at,
          dcsv.created_at,
          dcsv.expires_at
        FROM developer_client_secret_versions dcsv
        INNER JOIN oauth_clients oc
          ON oc.tenant_id = dcsv.tenant_id
         AND oc.client_id = dcsv.client_id
         AND oc.revoked_at IS NULL
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope) AS scopes
          FROM oauth_client_policies ocp
          CROSS JOIN LATERAL unnest(ocp.allowed_scopes) AS scope(scope)
          WHERE ocp.client_id = oc.id AND ocp.status = 'active'
        ) policy ON true
        WHERE dcsv.tenant_id = $1
          AND dcsv.revoked_at IS NULL
        UNION ALL
        SELECT oc.id,
          oc.client_id,
          oc.name,
          'client'::text AS status,
          'unknown'::text AS secret_last4,
          NULL::text AS owner_email,
          COALESCE(policy.scopes, ARRAY[]::text[]) AS scopes,
          oc.last_used_at,
          oc.created_at,
          NULL::timestamptz AS expires_at
        FROM oauth_clients oc
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope) AS scopes
          FROM oauth_client_policies ocp
          CROSS JOIN LATERAL unnest(ocp.allowed_scopes) AS scope(scope)
          WHERE ocp.client_id = oc.id AND ocp.status = 'active'
        ) policy ON true
        WHERE oc.tenant_id = $1
          AND oc.revoked_at IS NULL
          AND NOT EXISTS (
            SELECT 1
            FROM developer_client_secret_versions dcsv
            WHERE dcsv.tenant_id = oc.tenant_id
              AND dcsv.client_id = oc.client_id
          )
        ORDER BY created_at DESC
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

pub async fn session_policy(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Option<SessionPolicyRow>, AppError> {
    Ok(sqlx::query_as::<_, SessionPolicyRow>(
        r#"
        SELECT admin_session_ttl_hours
        FROM tenant_policies
        WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await?)
}

pub async fn mfa_policy(db: &PgPool, tenant_id: Uuid) -> Result<MfaPolicyRow, AppError> {
    Ok(sqlx::query_as::<_, MfaPolicyRow>(
        r#"
        SELECT COALESCE(mfa_policy, 'optional') AS policy
        FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
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
