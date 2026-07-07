use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::{db, grpc, policy};
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
    db::ensure_workspaces_belong(
        &mut tx,
        tenant_id,
        &input.workspace_ids,
        scope,
        auth.user_id,
    )
    .await?;
    tx.commit().await?;

    grpc::invitations::create_invitations(
        tenant_id,
        auth.user_id,
        scope,
        input.emails,
        policy::role_as_db(&input.role),
        input.workspace_ids,
    )
    .await
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

    let client_id =
        crate::domains::developer::grpc::revoke_secret_version(tenant_id, auth.user_id, version_id)
            .await?;
    grpc::audit::record_developer_secret_revoked(tenant_id, auth.user_id, version_id, &client_id)
        .await?;

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
