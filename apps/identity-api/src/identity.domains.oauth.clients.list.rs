use sqlx::PgPool;

use crate::domains::oauth::{logic::OAuthManagementAuth, service::types::OAuthClientsResult};
use crate::http::error::AppError;

/// List OAuth clients for a tenant.
pub async fn list_clients(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
) -> Result<OAuthClientsResult, AppError> {
    let tenant_id = super::require_oauth_management_tenant(db, auth).await?;
    let (current_scope_type, current_scope_id) =
        crate::domains::oauth::logic::resolve_management_scope(auth)?;
    let rows = sqlx::query(
        r#"
        SELECT
          oauth_clients.id,
          oauth_clients.client_id,
          oauth_clients.name,
          oauth_clients.redirect_uris,
          oauth_clients.created_at,
          oauth_clients.tenant_id,
          oauth_clients.owner_scope_type::text AS owner_scope_type,
          oauth_clients.owner_scope_id,
          oauth_clients.client_type::text AS client_type,
          oauth_clients.client_assertion_required,
          oauth_clients.client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          sa.principal_id AS service_account_principal_id,
          sa.workspace_id AS service_account_workspace_id,
          wm.role::text AS service_account_role
        FROM oauth_clients
        LEFT JOIN service_accounts sa ON sa.client_id = oauth_clients.client_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
         AND wm.status = 'active'
        WHERE oauth_clients.tenant_id = $1
          AND (
            (oauth_clients.owner_scope_type = $2::scope_type AND oauth_clients.owner_scope_id = $3)
            OR (oauth_clients.owner_scope_type = 'tenant' AND oauth_clients.owner_scope_id = $4)
          )
        ORDER BY oauth_clients.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(current_scope_type.as_str())
    .bind(current_scope_id)
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(OAuthClientsResult {
        clients: rows
            .into_iter()
            .map(|row| super::map_oauth_client_row(&row))
            .collect(),
    })
}
