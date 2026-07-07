#[path = "identity.domains.oauth.clients.create.rs"]
mod create;
#[path = "identity.domains.oauth.clients.list.rs"]
mod list;
#[path = "identity.domains.oauth.clients.revoke.rs"]
mod revoke;
#[path = "identity.domains.oauth.clients.service_accounts.rs"]
pub(super) mod service_accounts;

use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::http::error::AppError;

pub use create::create_client;
pub use list::list_clients;
pub use revoke::revoke_client;

async fn require_oauth_management_tenant(
    db: &sqlx::PgPool,
    auth: &(impl super::logic::OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
) -> Result<Uuid, AppError> {
    let tenant_id = super::logic::OAuthManagementAuth::tenant_id(auth).ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(tenant_id)
}

fn map_client_policy_row_with_client_id(
    row: &PgRow,
    client_id: Uuid,
) -> super::service::types::OAuthClientPolicyView {
    super::service::types::OAuthClientPolicyView {
        id: row.get("id"),
        client_id,
        scope_type: row.get("scope_type"),
        scope_id: row.get("scope_id"),
        allowed_scopes: row.get("allowed_scopes"),
        allowed_audiences: row.get("allowed_audiences"),
        allowed_resources: row.get("allowed_resources"),
        required_acr: row.get("required_acr"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    }
}
