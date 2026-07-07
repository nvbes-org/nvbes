use std::collections::BTreeMap;

use crate::database::Database;
use crate::domains::authz::{AdminScope, parse_identity_role};
use crate::domains::enterprise::{db, grpc, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;

pub(super) async fn ensure_member_manager(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<ActorAccess, AppError> {
    let access = require_actor_access(db, auth, tenant_id, scope).await?;
    if !policy::can_manage_members(
        policy::role_as_db(&access.role),
        &policy::grant_names(&access.grants),
    ) {
        return Err(AppError::forbidden(
            "members_grant_required",
            "Members access is required.",
        ));
    }
    if matches!(scope, AdminScope::Tenant) {
        crate::domains::enterprise::admin_elevation::require_active_admin_elevation(
            redis, auth, tenant_id,
        )
        .await?;
    }
    Ok(access)
}

pub(super) async fn require_actor_access(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<ActorAccess, AppError> {
    match scope {
        AdminScope::Tenant => {
            let row = db::actor_access(db, tenant_id, auth.user_id)
                .await?
                .ok_or_else(|| {
                    AppError::forbidden(
                        "tenant_management_denied",
                        "You do not have permission to manage this tenant.",
                    )
                })?;
            let role = policy::role_from_db(&row.role);
            let break_glass =
                active_break_glass_account(tenant_id, auth.user_id, auth.user_id).await?;
            Ok(ActorAccess {
                grants: policy::grants_for_role(&role),
                role,
                break_glass: break_glass.is_some(),
                break_glass_account: break_glass,
            })
        }
        AdminScope::Organization(org_id) => {
            let role_str = sqlx::query_scalar::<_, String>(
                r#"
                SELECT role::text
                FROM organization_memberships
                WHERE organization_id = $1
                  AND principal_id = $2
                  AND status = 'active'
                LIMIT 1
                "#,
            )
            .bind(org_id)
            .bind(auth.user_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| {
                AppError::forbidden(
                    "organization_management_denied",
                    "You do not have permission to manage this organization.",
                )
            })?;

            let parsed_role = parse_identity_role(&role_str)?;
            let ent_role = match parsed_role {
                nvbes_core::authz::IdentityRole::Owner => EnterpriseRole::Owner,
                nvbes_core::authz::IdentityRole::Admin => EnterpriseRole::Admin,
                nvbes_core::authz::IdentityRole::SecurityAdmin => EnterpriseRole::Admin,
                _ => EnterpriseRole::Member,
            };

            Ok(ActorAccess {
                grants: policy::grants_for_role(&ent_role),
                role: ent_role,
                break_glass: false,
                break_glass_account: None,
            })
        }
    }
}

pub(super) async fn fetch_user_view(
    db: &Database,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
) -> Result<EnterpriseUser, AppError> {
    let users = db::list_users(db, tenant_id, scope, actor_principal_id)
        .await?
        .into_iter()
        .filter(|row| row.id == user_id)
        .map(db::EnterpriseUserRow::into_view)
        .collect::<Vec<_>>();
    enrich_break_glass_accounts(tenant_id, actor_principal_id, users)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("enterprise_user_not_found", "Tenant member not found."))
}

pub(super) async fn enrich_break_glass_accounts(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    mut users: Vec<EnterpriseUser>,
) -> Result<Vec<EnterpriseUser>, AppError> {
    let principal_ids = users.iter().map(|user| user.id).collect::<Vec<_>>();
    let accounts = grpc::break_glass::list_active_break_glass_accounts(
        tenant_id,
        actor_principal_id,
        principal_ids,
    )
    .await?;
    let mut accounts = accounts
        .into_iter()
        .map(|account| (account.principal_id, account.account))
        .collect::<BTreeMap<_, _>>();
    for user in &mut users {
        user.break_glass = accounts.remove(&user.id);
    }
    Ok(users)
}

async fn active_break_glass_account(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    principal_id: Uuid,
) -> Result<Option<EnterpriseBreakGlassAccount>, AppError> {
    Ok(grpc::break_glass::list_active_break_glass_accounts(
        tenant_id,
        actor_principal_id,
        vec![principal_id],
    )
    .await?
    .into_iter()
    .next()
    .map(|account| account.account))
}

pub(super) struct ActorAccess {
    pub(super) role: EnterpriseRole,
    pub(super) grants: Vec<EnterpriseModuleGrant>,
    pub(super) break_glass: bool,
    pub(super) break_glass_account: Option<EnterpriseBreakGlassAccount>,
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
