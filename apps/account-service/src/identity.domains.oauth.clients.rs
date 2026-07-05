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

fn map_oauth_client_row(row: &PgRow) -> super::service::types::OAuthClientView {
    super::service::types::OAuthClientView {
        id: row.get("id"),
        client_id: row.get("client_id"),
        name: row.get("name"),
        redirect_uris: row.get("redirect_uris"),
        created_at: row.get("created_at"),
        last_used_at: row.get("last_used_at"),
        tenant_id: row.get("tenant_id"),
        owner_scope_type: row.get("owner_scope_type"),
        owner_scope_id: row.get("owner_scope_id"),
        client_type: row.get("client_type"),
        client_assertion_required: row.get("client_assertion_required"),
        requires_admin_consent: row.get("requires_admin_consent"),
        client_assertion_public_key_configured: row.get("client_assertion_public_key_configured"),
        service_account_principal_id: row.get("service_account_principal_id"),
        service_account_workspace_id: row.get("service_account_workspace_id"),
        service_account_role: row.get("service_account_role"),
    }
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
