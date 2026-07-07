use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::developer::grpc as developer_grpc;
use crate::domains::enterprise::{db, grpc};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::security_posture::security_posture;
use super::super::types::*;
use super::access::{
    all_grants, all_roles, default_security_signals, enrich_break_glass_accounts, metric,
    require_actor_access, security_signals, usage,
};
use super::developer_credentials::enterprise_developer_credential;
use super::policy_views::{
    mfa_policy_value, mfa_policy_view, session_policy_ttl, session_policy_view,
};

pub async fn get_context(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseContextResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    let access = require_actor_access(db, auth, tenant_id, scope).await?;
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
        break_glass: access.break_glass_account,
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
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    let (workspaces, users, storage) =
        db::usage_metrics(db, tenant_id, scope, auth.user_id).await?;
    Ok(EnterpriseOverviewResponse {
        metrics: vec![
            metric("workspaces", "Workspaces", workspaces),
            metric("users", "Users", users),
            metric("storage_used_bytes", "Storage used", storage),
        ],
        security_signals: default_security_signals(auth),
        recent_audit_events: grpc::audit::list_audit_events(tenant_id, auth.user_id, scope, 5)
            .await?,
    })
}

pub async fn list_users(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseUsersResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    let users = db::list_users(db, tenant_id, scope, auth.user_id)
        .await?
        .into_iter()
        .map(db::EnterpriseUserRow::into_view)
        .collect();
    Ok(EnterpriseUsersResponse {
        users: enrich_break_glass_accounts(tenant_id, auth.user_id, users).await?,
        invitations: db::list_invitations(db, tenant_id, scope, auth.user_id)
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
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    Ok(EnterpriseWorkspacesResponse {
        workspaces: db::list_workspaces(db, tenant_id, scope, auth.user_id)
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
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    Ok(EnterpriseDevelopersResponse {
        credentials: developer_grpc::list_credential_summaries(tenant_id, auth.user_id)
            .await?
            .into_iter()
            .map(enterprise_developer_credential)
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
    auth_session_ttl_hours: i64,
) -> Result<EnterprisePoliciesResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let policy_set = grpc::get_policy_set(tenant_id, auth.user_id).await?;
    let session_policy_ttl = session_policy_ttl(&policy_set)?;
    let mfa_policy = mfa_policy_value(&policy_set)?;
    Ok(EnterprisePoliciesResponse {
        policies: db::list_policies(db, tenant_id, auth.user_id)
            .await?
            .into_iter()
            .map(db::PolicySummaryRow::into_view)
            .collect(),
        session_policy: session_policy_view(session_policy_ttl, auth_session_ttl_hours),
        mfa_policy: mfa_policy_view(&mfa_policy),
    })
}

pub async fn get_security(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    auth_session_ttl_hours: i64,
) -> Result<EnterpriseSecurityResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let mut summary = db::security_summary(db, tenant_id, auth.user_id).await?;
    let trust_center = grpc::trust::get_trust_center(tenant_id, auth.user_id).await?;
    summary.verified_domain_count = trust_center
        .verified_domains
        .iter()
        .filter(|domain| domain.verified)
        .count() as i64;
    summary.sso_provider_count = trust_center.sso.active_providers;
    summary.stale_secret_count =
        developer_grpc::count_stale_secrets(tenant_id, auth.user_id).await?;
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
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    Ok(EnterpriseAuditEventsResponse {
        events: grpc::audit::list_audit_events(tenant_id, auth.user_id, scope, 50).await?,
        page: EnterprisePage {
            cursor: None,
            has_more: false,
        },
    })
}

pub async fn get_usage(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseUsageResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let (workspaces, users, storage) =
        db::usage_metrics(db, tenant_id, scope, auth.user_id).await?;
    Ok(EnterpriseUsageResponse {
        metrics: vec![
            usage("workspaces", "Workspaces", workspaces, None, "count"),
            usage("users", "Users", users, None, "count"),
            usage("storage_used_bytes", "Storage used", storage, None, "bytes"),
        ],
    })
}
