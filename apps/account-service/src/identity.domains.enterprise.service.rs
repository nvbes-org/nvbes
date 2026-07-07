#[path = "identity.domains.enterprise.service.access.rs"]
mod access;
#[path = "identity.domains.enterprise.service.break_glass.rs"]
mod break_glass;
#[path = "identity.domains.enterprise.service.developer_credentials.rs"]
mod developer_credentials;
#[path = "identity.domains.enterprise.service.mutations.rs"]
mod mutations;
#[path = "identity.domains.enterprise.service.policy_mutations.rs"]
mod policy_mutations;
#[path = "identity.domains.enterprise.service.policy_views.rs"]
mod policy_views;
#[path = "identity.domains.enterprise.service.reads.rs"]
mod reads;
#[path = "identity.domains.enterprise.service.user_mutations.rs"]
mod user_mutations;

use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
pub use break_glass::{activate_break_glass_account, revoke_break_glass_account};
pub use mutations::{create_invitations, revoke_developer_secret};
pub use policy_mutations::{update_mfa_policy, update_session_policy};
pub use reads::{
    get_context, get_overview, get_security, get_usage, list_audit_events, list_developers,
    list_policies, list_users, list_workspaces,
};
pub use user_mutations::{reactivate_user, suspend_user, update_user_access};
use uuid::Uuid;

pub async fn require_actor_access_for_enterprise(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    access::require_actor_access(db, auth, tenant_id, scope).await?;
    Ok(())
}

pub async fn grant_admin_elevation(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: crate::domains::enterprise::types::EnterpriseAdminElevationInput,
) -> Result<crate::domains::enterprise::types::EnterpriseAdminElevationResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let access = access::require_actor_access(db, auth, tenant_id, scope).await?;
    let break_glass_procedure = if access.break_glass {
        Some(break_glass::validate_break_glass_procedure(
            input.reason.clone(),
            input.procedure_reference.clone(),
        )?)
    } else {
        None
    };
    let response = crate::domains::enterprise::admin_elevation::grant_admin_elevation(
        redis,
        auth,
        &access.role,
        tenant_id,
        input,
        access.break_glass,
        break_glass_procedure,
    )
    .await?;
    Ok(response)
}
