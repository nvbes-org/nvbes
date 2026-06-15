use crate::database::Database;
use crate::domains::enterprise::db;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::security_posture::security_posture;
use super::super::types::*;
use super::access::{
    all_grants, all_roles, default_security_signals, metric, require_actor_access,
    security_signals, usage,
};

pub async fn get_context(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseContextResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let access = require_actor_access(db, auth, tenant_id).await?;
    let admin_elevation =
        crate::domains::enterprise::admin_elevation::active_admin_elevation(redis, auth, tenant_id)
            .await?
            .unwrap_or_else(crate::domains::enterprise::admin_elevation::inactive_admin_elevation);
    Ok(EnterpriseContextResponse {
        tenant_id,
        organization_id: auth.organization_id,
        workspace_id: auth.workspace_id,
        user_id: auth.user_id,
        role: access.role.clone(),
        module_grants: access.grants,
        admin_elevation,
        available_roles: all_roles(),
        available_module_grants: all_grants(),
    })
}

pub async fn get_overview(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseOverviewResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let (workspaces, users, storage) = db::usage_metrics(db, tenant_id).await?;
    Ok(EnterpriseOverviewResponse {
        metrics: vec![
            metric("workspaces", "Workspaces", workspaces),
            metric("users", "Users", users),
            metric("storage_used_bytes", "Storage used", storage),
        ],
        security_signals: default_security_signals(auth),
        recent_audit_events: db::list_audit_events(db, tenant_id, 5)
            .await?
            .into_iter()
            .map(db::AuditEventRow::into_view)
            .collect(),
    })
}

pub async fn list_users(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseUsersResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(EnterpriseUsersResponse {
        users: db::list_users(db, tenant_id)
            .await?
            .into_iter()
            .map(db::EnterpriseUserRow::into_view)
            .collect(),
        invitations: db::list_invitations(db, tenant_id)
            .await?
            .into_iter()
            .map(db::EnterpriseInvitationRow::into_view)
            .collect(),
        roles: all_roles(),
        module_grants: all_grants(),
        page: EnterprisePage {
            cursor: None,
            has_more: false,
        },
    })
}
pub async fn list_workspaces(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseWorkspacesResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(EnterpriseWorkspacesResponse {
        workspaces: db::list_workspaces(db, tenant_id)
            .await?
            .into_iter()
            .map(db::WorkspaceSummaryRow::into_view)
            .collect(),
        page: Some(EnterprisePage {
            cursor: None,
            has_more: false,
        }),
    })
}

pub async fn list_developers(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseDevelopersResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(EnterpriseDevelopersResponse {
        credentials: db::list_developers(db, tenant_id)
            .await?
            .into_iter()
            .map(db::DeveloperCredentialRow::into_view)
            .collect(),
        page: Some(EnterprisePage {
            cursor: None,
            has_more: false,
        }),
    })
}

pub async fn list_policies(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterprisePoliciesResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(EnterprisePoliciesResponse {
        policies: db::list_policies(db, tenant_id)
            .await?
            .into_iter()
            .map(db::PolicySummaryRow::into_view)
            .collect(),
    })
}

pub async fn get_security(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    auth_session_ttl_hours: i64,
) -> Result<EnterpriseSecurityResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let summary = db::security_summary(db, tenant_id).await?;
    Ok(EnterpriseSecurityResponse {
        signals: security_signals(&summary),
        posture: security_posture(&summary, auth_session_ttl_hours),
        mfa_required: summary.mfa_factor_count == 0,
        passkeys_enabled: summary.passkey_count > 0,
        recovery_approval_required: summary.high_risk_event_count > 0,
    })
}

pub async fn list_audit_events(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseAuditEventsResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(EnterpriseAuditEventsResponse {
        events: db::list_audit_events(db, tenant_id, 50)
            .await?
            .into_iter()
            .map(db::AuditEventRow::into_view)
            .collect(),
        page: EnterprisePage {
            cursor: None,
            has_more: false,
        },
    })
}

pub async fn get_billing(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseBillingResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let row = db::billing_summary(db, tenant_id).await?;
    Ok(EnterpriseBillingResponse {
        plan: EnterpriseBillingPlan {
            code: row
                .as_ref()
                .map(|row| row.code.clone())
                .unwrap_or_else(|| "none".to_string()),
            name: row
                .as_ref()
                .map(|row| row.name.clone())
                .unwrap_or_else(|| "No plan".to_string()),
            status: row
                .as_ref()
                .map(|row| row.status.clone())
                .unwrap_or_else(|| "inactive".to_string()),
            currency: "usd".to_string(),
            monthly_price_cents: 0,
        },
        invoices: db::list_invoices(db, tenant_id)
            .await?
            .into_iter()
            .map(db::InvoiceRow::into_view)
            .collect(),
        billing_email: row.and_then(|row| row.billing_email),
    })
}

pub async fn get_usage(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseUsageResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let (workspaces, users, storage) = db::usage_metrics(db, tenant_id).await?;
    Ok(EnterpriseUsageResponse {
        metrics: vec![
            usage("workspaces", "Workspaces", workspaces, None, "count"),
            usage("users", "Users", users, None, "count"),
            usage("storage_used_bytes", "Storage used", storage, None, "bytes"),
        ],
    })
}
