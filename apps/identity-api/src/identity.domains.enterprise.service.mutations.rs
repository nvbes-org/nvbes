use crate::database::Database;
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;
use super::access::{ensure_member_manager, fetch_user_view};

pub async fn create_invitations(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: EnterpriseInvitationInput,
) -> Result<EnterpriseInvitationsResponse, AppError> {
    let actor_access = ensure_member_manager(db, auth, tenant_id).await?;
    ensure_owner_role_allowed(&actor_access, policy::role_as_db(&input.role))?;
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
            if db::has_pending_invitation(&mut tx, tenant_id, *workspace_id, &email).await? {
                return Err(AppError::conflict(
                    "invitation_pending",
                    "A pending invitation already exists for this email and workspace.",
                ));
            }
            let token = crate::domains::auth::password::generate_token("gxi");
            let token_hash = crate::domains::auth::password::token_hash(&token);
            let row = db::insert_invitation(
                &mut tx,
                tenant_id,
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
    let actor_access = ensure_member_manager(db, auth, tenant_id).await?;
    ensure_owner_role_allowed(&actor_access, policy::role_as_db(&input.role))?;
    if input.workspace_ids.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one workspace ID is required.",
        ));
    }
    let mut tx = db.begin().await?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids).await?;
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
    let actor_access = ensure_member_manager(db, auth, tenant_id).await?;
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
        user: fetch_user_view(db, tenant_id, user_id).await?,
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
        user: fetch_user_view(db, tenant_id, user_id).await?,
    })
}
