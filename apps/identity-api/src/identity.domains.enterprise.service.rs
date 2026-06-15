#[path = "identity.domains.enterprise.service.access.rs"]
mod access;
#[path = "identity.domains.enterprise.service.mutations.rs"]
mod mutations;
#[path = "identity.domains.enterprise.service.policy_mutations.rs"]
mod policy_mutations;
#[path = "identity.domains.enterprise.service.reads.rs"]
mod reads;
#[path = "identity.domains.enterprise.service.user_mutations.rs"]
mod user_mutations;

use crate::database::Database;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
pub use mutations::{create_invitations, revoke_developer_secret};
pub use policy_mutations::{update_mfa_policy, update_session_policy};
pub use reads::{
    get_billing, get_context, get_overview, get_security, get_usage, list_audit_events,
    list_developers, list_policies, list_users, list_workspaces,
};
pub use user_mutations::{reactivate_user, suspend_user, update_user_access};
use uuid::Uuid;

pub async fn require_actor_access_for_enterprise(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    access::require_actor_access(db, auth, tenant_id).await?;
    Ok(())
}

pub async fn grant_admin_elevation(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: crate::domains::enterprise::types::EnterpriseAdminElevationInput,
) -> Result<crate::domains::enterprise::types::EnterpriseAdminElevationResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let access = access::require_actor_access(db, auth, tenant_id).await?;
    crate::domains::enterprise::admin_elevation::grant_admin_elevation(
        redis,
        auth,
        &access.role,
        tenant_id,
        input,
    )
    .await
}
