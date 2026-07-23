use nvbes_core::pagination::{KeysetCursor, decode_cursor, encode_cursor, page_from_rows};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::cloud_boundary::workspace_port;
use crate::domains::oauth::{
    logic::OAuthManagementAuth,
    service::types::{OAuthClientView, OAuthClientsResult},
};
use crate::http::error::AppError;

/// List OAuth clients for a tenant.
pub async fn list_clients(
    db: &PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<OAuthClientsResult, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    let decoded = cursor
        .as_deref()
        .map(decode_cursor::<KeysetCursor>)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let tenant_id = super::require_oauth_consent_tenant(db, auth).await?;
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
          AND EXISTS (
            SELECT 1
            FROM oauth_consents
            WHERE oauth_consents.client_id = oauth_clients.id
              AND oauth_consents.principal_id = $2
              AND oauth_consents.tenant_id = $1
              AND oauth_consents.revoked_at IS NULL
              AND (oauth_consents.expires_at IS NULL OR oauth_consents.expires_at > NOW())
          )
          AND ($3::timestamp with time zone IS NULL OR (oauth_clients.created_at, oauth_clients.id) < ($3, $4))
        ORDER BY oauth_clients.created_at DESC, oauth_clients.id DESC
        LIMIT $5
        "#,
    )
    .bind(tenant_id)
    .bind(OAuthManagementAuth::user_id(auth))
    .bind(decoded.as_ref().map(|c| c.created_at))
    .bind(decoded.as_ref().map(|c| c.id))
    .bind((limit + 1) as i64)
    .fetch_all(db)
    .await?;

    let page = page_from_rows(rows, limit, |row| KeysetCursor {
        created_at: row.get("created_at"),
        id: row.get("id"),
    });
    let mut clients = Vec::with_capacity(page.items.len());
    for row in page.items {
        clients.push(map_oauth_client_row(tenant_id, row).await?);
    }

    let next_cursor = page
        .next_cursor
        .map(|cursor| encode_cursor(&cursor))
        .transpose()
        .map_err(|_| {
            AppError::internal("pagination_error", "Failed to encode pagination cursor.")
        })?;
    Ok(OAuthClientsResult {
        clients,
        next_cursor,
        has_more: page.has_more,
    })
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
