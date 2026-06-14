use std::collections::BTreeMap;

use crate::database::Database;
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::types::*;

pub async fn get_context(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseContextResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let access = require_actor_access(db, auth, tenant_id).await?;
    Ok(EnterpriseContextResponse {
        tenant_id,
        organization_id: auth.organization_id,
        workspace_id: auth.workspace_id,
        user_id: auth.user_id,
        role: access.role.clone(),
        module_grants: access.grants,
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

pub async fn create_invitations(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: EnterpriseInvitationInput,
) -> Result<EnterpriseInvitationsResponse, AppError> {
    ensure_member_manager(db, auth, tenant_id).await?;
    if input.workspace_ids.is_empty() || input.emails.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one email and one workspace ID are required.",
        ));
    }

    let mut tx = db.begin().await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids).await?;
    let mut invitations = Vec::new();
    for raw_email in input.emails {
        let email = crate::domains::auth::password::normalize_email(&raw_email);
        crate::domains::auth::password::validate_email(&email)?;
        for workspace_id in &input.workspace_ids {
            if db::has_pending_invitation(&mut tx, *workspace_id, &email).await? {
                return Err(AppError::conflict(
                    "invitation_pending",
                    "A pending invitation already exists for this email and workspace.",
                ));
            }
            let token = crate::domains::auth::password::generate_token("gxi");
            let token_hash = crate::domains::auth::password::token_hash(&token);
            let row = db::insert_invitation(
                &mut tx,
                *workspace_id,
                &email,
                policy::role_as_db(&input.role),
                &token_hash,
            )
            .await?;
            db::insert_audit(
                &mut tx,
                tenant_id,
                auth.user_id,
                "enterprise.member.invited",
                "workspace_invitation",
                Some(row.id),
                serde_json::json!({"email": email, "role": policy::role_as_db(&input.role), "workspace_id": workspace_id}),
            )
            .await?;
            invitations.push(row.into_view());
        }
    }
    tx.commit().await?;
    Ok(EnterpriseInvitationsResponse { invitations })
}

pub async fn update_user_access(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseAccessUpdateInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    ensure_member_manager(db, auth, tenant_id).await?;
    if input.workspace_ids.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one workspace ID is required.",
        ));
    }
    let mut tx = db.begin().await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids).await?;
    let current_role = db::target_role(&mut tx, tenant_id, user_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("enterprise_user_not_found", "Tenant member not found.")
        })?;
    let owner_count = db::active_owner_count(&mut tx, tenant_id).await?;
    if policy::is_last_owner_removal(owner_count, &current_role, policy::role_as_db(&input.role)) {
        return Err(AppError::conflict(
            "last_owner_removal",
            "The final tenant owner cannot be downgraded.",
        ));
    }
    db::replace_access(
        &mut tx,
        tenant_id,
        user_id,
        &input.workspace_ids,
        policy::role_as_db(&input.role),
    )
    .await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.member.access_updated",
        "principal",
        Some(user_id),
        serde_json::json!({
            "before": {"role": current_role},
            "after": {"role": policy::role_as_db(&input.role), "workspace_ids": input.workspace_ids}
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(EnterpriseAccessUpdateResponse {
        user: fetch_user_view(db, tenant_id, user_id).await?,
    })
}

pub async fn suspend_user(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseSuspendInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    ensure_member_manager(db, auth, tenant_id).await?;
    let mut tx = db.begin().await?;
    let current_role = db::target_role(&mut tx, tenant_id, user_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("enterprise_user_not_found", "Tenant member not found.")
        })?;
    let owner_count = db::active_owner_count(&mut tx, tenant_id).await?;
    if policy::is_last_owner_removal(owner_count, &current_role, "suspended") {
        return Err(AppError::conflict(
            "last_owner_removal",
            "The final tenant owner cannot be suspended.",
        ));
    }
    db::set_tenant_memberships_status(&mut tx, tenant_id, user_id, "suspended", None).await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.member.suspended",
        "principal",
        Some(user_id),
        serde_json::json!({"before": {"role": current_role}, "reason": input.reason}),
    )
    .await?;
    tx.commit().await?;
    Ok(EnterpriseAccessUpdateResponse {
        user: fetch_user_view(db, tenant_id, user_id).await?,
    })
}

pub async fn reactivate_user(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseReactivateInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    ensure_member_manager(db, auth, tenant_id).await?;
    let mut tx = db.begin().await?;
    if let Some(workspace_ids) = input.workspace_ids.as_ref() {
        if workspace_ids.is_empty() {
            return Err(AppError::bad_request(
                "validation_failed",
                "Workspace IDs cannot be empty when provided.",
            ));
        }
        db::ensure_workspaces_belong(&mut tx, tenant_id, workspace_ids).await?;
    }
    db::set_tenant_memberships_status(
        &mut tx,
        tenant_id,
        user_id,
        "active",
        input.workspace_ids.as_deref(),
    )
    .await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.member.reactivated",
        "principal",
        Some(user_id),
        serde_json::json!({"reason": input.reason, "workspace_ids": input.workspace_ids}),
    )
    .await?;
    tx.commit().await?;
    Ok(EnterpriseAccessUpdateResponse {
        user: fetch_user_view(db, tenant_id, user_id).await?,
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
    _: &Database,
    _: &AuthContext,
    _: Uuid,
) -> Result<EnterpriseDevelopersResponse, AppError> {
    Ok(EnterpriseDevelopersResponse {
        credentials: Vec::new(),
        page: None,
    })
}

pub async fn list_policies(
    _: &Database,
    _: &AuthContext,
    _: Uuid,
) -> Result<EnterprisePoliciesResponse, AppError> {
    Ok(EnterprisePoliciesResponse {
        policies: Vec::new(),
    })
}

pub async fn get_security(
    _: &Database,
    auth: &AuthContext,
    _: Uuid,
) -> Result<EnterpriseSecurityResponse, AppError> {
    Ok(EnterpriseSecurityResponse {
        signals: default_security_signals(auth),
        mfa_required: false,
        passkeys_enabled: false,
        recovery_approval_required: false,
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
        invoices: Vec::new(),
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

async fn ensure_member_manager(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
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
    Ok(())
}

async fn require_actor_access(
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

async fn fetch_user_view(
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

struct ActorAccess {
    role: EnterpriseRole,
    grants: Vec<EnterpriseModuleGrant>,
}

fn all_roles() -> Vec<EnterpriseRole> {
    vec![
        EnterpriseRole::Owner,
        EnterpriseRole::Admin,
        EnterpriseRole::Member,
        EnterpriseRole::Viewer,
    ]
}

fn all_grants() -> Vec<EnterpriseModuleGrant> {
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

fn metric(key: &str, label: &str, value: i64) -> EnterpriseOverviewMetric {
    EnterpriseOverviewMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
        delta_percent: None,
    }
}

fn usage(
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

fn default_security_signals(auth: &AuthContext) -> Vec<EnterpriseSecuritySignal> {
    let status = if auth.mfa_enabled { "ok" } else { "attention" };
    vec![EnterpriseSecuritySignal {
        key: "mfa".to_string(),
        label: "Multi-factor authentication".to_string(),
        status: status.to_string(),
        severity: if auth.mfa_enabled { "info" } else { "medium" }.to_string(),
        details: Some(BTreeMap::new()),
    }]
}
