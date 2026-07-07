use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domains::{
    cloud::workspace_port,
    oauth::{
        logic::OAuthManagementAuth,
        service::types::{OAuthClientView, OAuthClientsResult},
    },
};
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
          oauth_clients.last_used_at,
          oauth_clients.tenant_id,
          oauth_clients.owner_scope_type::text AS owner_scope_type,
          oauth_clients.owner_scope_id,
          oauth_clients.client_type::text AS client_type,
          oauth_clients.client_assertion_required,
          oauth_clients.requires_admin_consent,
          oauth_clients.client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          sa.principal_id AS service_account_principal_id,
          sa.workspace_id AS service_account_workspace_id
        FROM oauth_clients
        LEFT JOIN service_accounts sa ON sa.client_id = oauth_clients.client_id
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

    let mut clients = Vec::with_capacity(rows.len());
    for row in rows {
        clients.push(map_oauth_client_row(tenant_id, row).await?);
    }

    Ok(OAuthClientsResult { clients })
}

async fn map_oauth_client_row(tenant_id: Uuid, row: PgRow) -> Result<OAuthClientView, AppError> {
    let service_account_principal_id: Option<Uuid> = row.get("service_account_principal_id");
    let service_account_workspace_id: Option<Uuid> = row.get("service_account_workspace_id");
    let service_account_role = match (service_account_principal_id, service_account_workspace_id) {
        (Some(principal_id), Some(workspace_id)) => {
            workspace_port::list_workspace_members(Some(tenant_id), workspace_id, principal_id)
                .await?
                .into_iter()
                .find(|member| member.principal_id == principal_id && member.active)
                .map(|member| member.role)
        }
        _ => None,
    };

    Ok(OAuthClientView {
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
        service_account_principal_id,
        service_account_workspace_id,
        service_account_role,
    })
}
