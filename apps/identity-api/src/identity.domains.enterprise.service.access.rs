use std::collections::BTreeMap;

use crate::database::Database;
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;

pub(super) async fn ensure_member_manager(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<ActorAccess, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let access = require_actor_access(db, auth, tenant_id).await?;
    if !policy::can_manage_members(
        policy::role_as_db(&access.role),
        &policy::grant_names(&access.grants),
    ) {
        return Err(AppError::forbidden(
            "members_grant_required",
            "Members access is required.",
        ));
    }
    Ok(access)
}

pub(super) async fn require_actor_access(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<ActorAccess, AppError> {
    let row = db::actor_access(db, tenant_id, auth.user_id)
        .await?
        .ok_or_else(|| {
            AppError::forbidden(
                "tenant_management_denied",
                "You do not have permission to manage this tenant.",
            )
        })?;
    let role = policy::role_from_db(&row.role);
    Ok(ActorAccess {
        grants: policy::grants_for_role(&role),
        role,
    })
}

pub(super) async fn fetch_user_view(
    db: &Database,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<EnterpriseUser, AppError> {
    db::list_users(db, tenant_id)
        .await?
        .into_iter()
        .find(|row| row.id == user_id)
        .map(db::EnterpriseUserRow::into_view)
        .ok_or_else(|| AppError::not_found("enterprise_user_not_found", "Tenant member not found."))
}

pub(super) struct ActorAccess {
    pub(super) role: EnterpriseRole,
    pub(super) grants: Vec<EnterpriseModuleGrant>,
}

pub(super) fn all_roles() -> Vec<EnterpriseRole> {
    vec![
        EnterpriseRole::Owner,
        EnterpriseRole::Admin,
        EnterpriseRole::Member,
        EnterpriseRole::Viewer,
    ]
}

pub(super) fn all_grants() -> Vec<EnterpriseModuleGrant> {
    vec![
        EnterpriseModuleGrant::Members,
        EnterpriseModuleGrant::Workspaces,
        EnterpriseModuleGrant::Developers,
        EnterpriseModuleGrant::Policies,
        EnterpriseModuleGrant::Security,
        EnterpriseModuleGrant::Billing,
        EnterpriseModuleGrant::Audit,
        EnterpriseModuleGrant::Drive,
    ]
}

pub(super) fn metric(key: &str, label: &str, value: i64) -> EnterpriseOverviewMetric {
    EnterpriseOverviewMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
        delta_percent: None,
    }
}

pub(super) fn usage(
    key: &str,
    label: &str,
    value: i64,
    limit: Option<i64>,
    unit: &str,
) -> EnterpriseUsageMetric {
    EnterpriseUsageMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
        limit,
        unit: unit.to_string(),
    }
}

pub(super) fn default_security_signals(auth: &AuthContext) -> Vec<EnterpriseSecuritySignal> {
    let status = if auth.mfa_enabled { "ok" } else { "attention" };
    vec![EnterpriseSecuritySignal {
        key: "mfa".to_string(),
        label: "Multi-factor authentication".to_string(),
        status: status.to_string(),
        severity: if auth.mfa_enabled { "info" } else { "medium" }.to_string(),
        details: Some(BTreeMap::new()),
    }]
}

pub(super) fn security_signals(summary: &db::SecuritySummaryRow) -> Vec<EnterpriseSecuritySignal> {
    vec![
        EnterpriseSecuritySignal {
            key: "mfa_adoption".to_string(),
            label: "Active MFA factors".to_string(),
            status: if summary.mfa_factor_count > 0 {
                "ok"
            } else {
                "attention"
            }
            .to_string(),
            severity: if summary.mfa_factor_count > 0 {
                "info"
            } else {
                "medium"
            }
            .to_string(),
            details: Some(BTreeMap::from([(
                "active_factor_count".to_string(),
                serde_json::json!(summary.mfa_factor_count),
            )])),
        },
        EnterpriseSecuritySignal {
            key: "risk_events".to_string(),
            label: "High risk events".to_string(),
            status: if summary.high_risk_event_count == 0 {
                "ok"
            } else {
                "attention"
            }
            .to_string(),
            severity: if summary.high_risk_event_count == 0 {
                "info"
            } else {
                "high"
            }
            .to_string(),
            details: Some(BTreeMap::from([(
                "high_risk_event_count".to_string(),
                serde_json::json!(summary.high_risk_event_count),
            )])),
        },
    ]
}
