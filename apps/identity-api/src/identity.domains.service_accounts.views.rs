use sqlx::Row;
use uuid::Uuid;

use crate::domains::{
    oauth,
    service_accounts::types::{ServiceAccountClientView, ServiceAccountView},
};

pub fn service_account_view_from_row(row: sqlx::postgres::PgRow) -> ServiceAccountView {
    let oauth_clients = row
        .get::<Option<Uuid>, _>("oauth_client_uuid")
        .map(|id| ServiceAccountClientView {
            id,
            client_id: row.get("client_id"),
            name: row.get("oauth_client_name"),
            created_at: row.get("oauth_client_created_at"),
            last_used_at: row.get("oauth_client_last_used_at"),
            revoked_at: row.get("oauth_client_revoked_at"),
            client_assertion_required: row.get("client_assertion_required"),
            client_assertion_public_key_configured: row
                .get("client_assertion_public_key_configured"),
            allowed_scopes: row
                .get::<Option<Vec<String>>, _>("allowed_scopes")
                .unwrap_or_default(),
            allowed_audiences: row
                .get::<Option<Vec<String>>, _>("allowed_audiences")
                .unwrap_or_default(),
            allowed_resources: row
                .get::<Option<Vec<String>>, _>("allowed_resources")
                .unwrap_or_default(),
            required_acr: row
                .get::<Option<String>, _>("required_acr")
                .unwrap_or_else(|| "aal1".to_string()),
        })
        .into_iter()
        .collect();

    ServiceAccountView {
        principal_id: row.get("principal_id"),
        tenant_id: row.get("tenant_id"),
        organization_id: row.get("organization_id"),
        workspace_id: row.get("workspace_id"),
        name: row.get("name"),
        description: row.get("description"),
        role: row.get("role"),
        status: row.get("principal_status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        oauth_clients,
    }
}

pub fn service_account_client_view_from_oauth_result(
    result: &oauth::service::CreateOAuthClientResult,
) -> ServiceAccountClientView {
    ServiceAccountClientView {
        id: result.client.id,
        client_id: result.client.client_id.clone(),
        name: result.client.name.clone(),
        created_at: result.client.created_at,
        last_used_at: result.client.last_used_at,
        revoked_at: None,
        client_assertion_required: result.client.client_assertion_required,
        client_assertion_public_key_configured: result
            .client
            .client_assertion_public_key_configured,
        allowed_scopes: result.policy.allowed_scopes.clone(),
        allowed_audiences: result.policy.allowed_audiences.clone(),
        allowed_resources: result.policy.allowed_resources.clone(),
        required_acr: result.policy.required_acr.clone(),
    }
}
