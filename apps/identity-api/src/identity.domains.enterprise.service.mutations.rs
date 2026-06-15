use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;
use super::access::ensure_member_manager;

pub async fn create_invitations(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: EnterpriseInvitationInput,
) -> Result<EnterpriseInvitationsResponse, AppError> {
    let scope = resolve_admin_scope(&db, auth, tenant_id, auth.organization_id).await?;
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;
    ensure_owner_role_allowed(&actor_access, policy::role_as_db(&input.role))?;
    if input.workspace_ids.is_empty() || input.emails.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one email and one workspace ID are required.",
        ));
    }

    let mut tx = db.begin().await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids, scope).await?;
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

pub async fn revoke_developer_secret(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    version_id: Uuid,
) -> Result<EnterpriseDevelopersResponse, AppError> {
    let scope = resolve_admin_scope(&db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let access = super::access::require_actor_access(db, auth, tenant_id, scope).await?;
    if !policy::can_manage_developers(
        policy::role_as_db(&access.role),
        &policy::grant_names(&access.grants),
    ) {
        return Err(AppError::forbidden(
            "developers_grant_required",
            "Developers access is required.",
        ));
    }
    crate::domains::enterprise::admin_elevation::require_active_admin_elevation(
        redis, auth, tenant_id,
    )
    .await?;

    let mut tx = db.begin().await?;
    let client_id = db::revoke_developer_secret_version(&mut tx, tenant_id, version_id).await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.developer_secret.revoked",
        "developer_client_secret_version",
        Some(version_id),
        serde_json::json!({"client_id": client_id}),
    )
    .await?;
    tx.commit().await?;

    super::reads::list_developers(db, auth, tenant_id).await
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
