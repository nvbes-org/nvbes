use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;
use super::access::{ensure_member_manager, fetch_user_view};

async fn ensure_user_in_organization(
    db: &Database,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM organization_memberships WHERE organization_id = $1 AND principal_id = $2 AND status = 'active')",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;

    if !exists {
        return Err(AppError::forbidden(
            "user_not_in_organization",
            "This user is not a member of your organization.",
        ));
    }
    Ok(())
}

pub async fn update_user_access(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseAccessUpdateInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    let scope = resolve_admin_scope(&db, auth, tenant_id, auth.organization_id).await?;
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;
    ensure_owner_role_allowed(&actor_access, policy::role_as_db(&input.role))?;

    if let AdminScope::Organization(org_id) = scope {
        ensure_user_in_organization(db, user_id, org_id).await?;
    }

    if input.workspace_ids.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one workspace ID is required.",
        ));
    }
    let mut tx = db.begin().await?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids, scope).await?;
    let current_role = db::target_role(&mut tx, tenant_id, user_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("enterprise_user_not_found", "Tenant member not found.")
        })?;
    ensure_owner_target_allowed(&actor_access, &current_role)?;
    let ownerless_count = db::ownerless_workspace_count_after_access(
        &mut tx,
        tenant_id,
        user_id,
        &input.workspace_ids,
        policy::role_as_db(&input.role),
    )
    .await?;
    if ownerless_count > 0 {
        return Err(AppError::conflict(
            "last_owner_removal",
            "Every workspace must retain at least one active owner.",
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
        user: fetch_user_view(db, tenant_id, user_id, scope).await?,
    })
}

pub async fn suspend_user(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseSuspendInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    let scope = resolve_admin_scope(&db, auth, tenant_id, auth.organization_id).await?;
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;

    if let AdminScope::Organization(org_id) = scope {
        ensure_user_in_organization(db, user_id, org_id).await?;
    }

    let mut tx = db.begin().await?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    let current_role = db::target_role(&mut tx, tenant_id, user_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("enterprise_user_not_found", "Tenant member not found.")
        })?;
    ensure_owner_target_allowed(&actor_access, &current_role)?;
    let ownerless_count =
        db::ownerless_workspace_count_after_status(&mut tx, tenant_id, user_id).await?;
    if ownerless_count > 0 {
        return Err(AppError::conflict(
            "last_owner_removal",
            "Every workspace must retain at least one active owner.",
        ));
    }
    let changed =
        db::set_tenant_memberships_status(&mut tx, tenant_id, user_id, "suspended", None).await?;
    if changed == 0 {
        return Err(AppError::not_found(
            "enterprise_user_not_found",
            "Tenant member not found.",
        ));
    }
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
        user: fetch_user_view(db, tenant_id, user_id, scope).await?,
    })
}

pub async fn reactivate_user(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseReactivateInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    let scope = resolve_admin_scope(&db, auth, tenant_id, auth.organization_id).await?;
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;

    if let AdminScope::Organization(org_id) = scope {
        ensure_user_in_organization(db, user_id, org_id).await?;
    }

    let mut tx = db.begin().await?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    let current_role = db::target_role_for_lifecycle(&mut tx, tenant_id, user_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("enterprise_user_not_found", "Tenant member not found.")
        })?;
    ensure_owner_target_allowed(&actor_access, &current_role)?;
    if let Some(workspace_ids) = input.workspace_ids.as_ref() {
        if workspace_ids.is_empty() {
            return Err(AppError::bad_request(
                "validation_failed",
                "Workspace IDs cannot be empty when provided.",
            ));
        }
        db::ensure_workspaces_belong(&mut tx, tenant_id, workspace_ids, scope).await?;
    }
    let changed = db::set_tenant_memberships_status(
        &mut tx,
        tenant_id,
        user_id,
        "active",
        input.workspace_ids.as_deref(),
    )
    .await?;
    if changed == 0 {
        return Err(AppError::not_found(
            "enterprise_user_not_found",
            "Tenant member not found.",
        ));
    }
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
        user: fetch_user_view(db, tenant_id, user_id, scope).await?,
    })
}

fn ensure_owner_role_allowed(
    actor_access: &super::access::ActorAccess,
    requested_role: &str,
) -> Result<(), AppError> {
    if requested_role == "owner" && policy::role_as_db(&actor_access.role) != "owner" {
        return Err(AppError::forbidden(
            "owner_required",
            "Only owners can grant owner access.",
        ));
    }
    Ok(())
}

fn ensure_owner_target_allowed(
    actor_access: &super::access::ActorAccess,
    target_role: &str,
) -> Result<(), AppError> {
    if target_role == "owner" && policy::role_as_db(&actor_access.role) != "owner" {
        return Err(AppError::forbidden(
            "owner_required",
            "Only owners can modify owner access.",
        ));
    }
    Ok(())
}
