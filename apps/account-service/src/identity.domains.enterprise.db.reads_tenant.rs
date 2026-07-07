use std::collections::HashSet;

use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::records::{PolicySummaryRow, SecuritySummaryRow};
use crate::{domains::cloud::workspace_port, http::error::AppError};

pub async fn list_policies(
    _db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<Vec<PolicySummaryRow>, AppError> {
    let mut policies = workspace_port::list_tenant_workspaces(tenant_id, None, actor_principal_id)
        .await?
        .into_iter()
        .map(|workspace| PolicySummaryRow {
            id: workspace.workspace_id,
            name: format!("Workspace policy: {}", workspace.name),
            category: "workspace".to_string(),
            enabled: true,
            configuration: workspace_policy_configuration(&workspace),
            updated_at: workspace.updated_at,
        })
        .collect::<Vec<_>>();
    policies.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(policies)
}

pub async fn security_summary(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<SecuritySummaryRow, AppError> {
    let base = sqlx::query_as::<_, SecurityBaseRow>(
        r#"
        SELECT
          COUNT(DISTINCT mf.id) FILTER (WHERE mf.status = 'active')::bigint AS mfa_factor_count,
          COUNT(DISTINCT mf.id) FILTER (WHERE mf.status = 'active' AND mf.factor_type = 'webauthn')::bigint AS passkey_count,
          COUNT(DISTINCT re.id) FILTER (WHERE re.risk_score >= 0.7)::bigint AS high_risk_event_count
        FROM tenant_memberships tm
        LEFT JOIN mfa_factors mf ON mf.principal_id = tm.principal_id
        LEFT JOIN risk_events re ON re.principal_id = tm.principal_id
        WHERE tm.tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await?;
    let active_members = active_tenant_member_mfa(db, tenant_id).await?;
    let admin_principals = active_workspace_admins(tenant_id, actor_principal_id).await?;
    let admin_without_mfa_count = active_members
        .into_iter()
        .filter(|member| admin_principals.contains(&member.principal_id) && !member.mfa_enabled)
        .count() as i64;

    Ok(SecuritySummaryRow {
        mfa_factor_count: base.mfa_factor_count,
        passkey_count: base.passkey_count,
        high_risk_event_count: base.high_risk_event_count,
        admin_without_mfa_count,
        verified_domain_count: 0,
        sso_provider_count: 0,
        stale_secret_count: 0,
    })
}

fn workspace_policy_configuration(workspace: &workspace_port::CloudWorkspaceSummary) -> Value {
    serde_json::json!({
        "member_can_create_share_links": workspace.member_can_create_share_links,
        "require_admin_approval_for_member_share": workspace.require_admin_approval_for_member_share,
        "default_share_link_ttl_days": workspace.default_share_link_ttl_days,
        "max_share_link_ttl_days": workspace.max_share_link_ttl_days,
        "required_acr": workspace.required_acr.clone().unwrap_or_else(|| "aal1".to_string()),
    })
}

async fn active_workspace_admins(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<HashSet<Uuid>, AppError> {
    let mut admin_principals = HashSet::new();
    for workspace in
        workspace_port::list_tenant_workspaces(tenant_id, None, actor_principal_id).await?
    {
        for member in workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            actor_principal_id,
        )
        .await?
        {
            if member.active && matches!(member.role.as_str(), "owner" | "admin") {
                admin_principals.insert(member.principal_id);
            }
        }
    }
    Ok(admin_principals)
}

async fn active_tenant_member_mfa(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<ActiveTenantMemberMfaRow>, AppError> {
    Ok(sqlx::query_as::<_, ActiveTenantMemberMfaRow>(
        r#"
        SELECT
          tm.principal_id,
          EXISTS (
            SELECT 1 FROM mfa_factors mf
            WHERE mf.principal_id = tm.principal_id
              AND mf.status = 'active'
          ) AS mfa_enabled
        FROM tenant_memberships tm
        WHERE tm.tenant_id = $1
          AND tm.status = 'active'
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

#[derive(Debug, FromRow)]
struct SecurityBaseRow {
    mfa_factor_count: i64,
    passkey_count: i64,
    high_risk_event_count: i64,
}

#[derive(Debug, FromRow)]
struct ActiveTenantMemberMfaRow {
    principal_id: Uuid,
    mfa_enabled: bool,
}
