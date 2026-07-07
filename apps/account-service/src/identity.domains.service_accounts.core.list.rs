use std::collections::HashMap;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::{
        authz::WorkspaceAccess,
        cloud::workspace_port,
        service_accounts::{
            types::{ServiceAccountClientView, ServiceAccountView, ServiceAccountsResult},
            views::service_account_client_view_from_row,
        },
    },
    http::error::AppError,
};

pub async fn list_service_accounts(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<ServiceAccountsResult, AppError> {
    let workspace =
        workspace_port::get_workspace(access.tenant_id, access.workspace_id, access.auth.user_id)
            .await?;
    let member_roles = workspace_port::list_workspace_members(
        access.tenant_id,
        access.workspace_id,
        access.auth.user_id,
    )
    .await?
    .into_iter()
    .filter(|member| member.active)
    .map(|member| (member.principal_id, member.role))
    .collect::<HashMap<_, _>>();

    let rows = sqlx::query(
        r#"
        SELECT
          sa.principal_id,
          sa.tenant_id,
          sa.workspace_id,
          sa.name,
          sa.description,
          p.status::text AS principal_status,
          sa.created_at,
          sa.updated_at,
          oc.id AS oauth_client_uuid,
          oc.client_id,
          oc.name AS oauth_client_name,
          oc.created_at AS oauth_client_created_at,
          oc.last_used_at AS oauth_client_last_used_at,
          oc.revoked_at AS oauth_client_revoked_at,
          oc.client_assertion_required,
          oc.client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          ocp.allowed_scopes,
          ocp.allowed_audiences,
          ocp.allowed_resources,
          ocp.required_acr::text AS required_acr
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        LEFT JOIN oauth_clients oc
          ON oc.client_id = sa.client_id
        LEFT JOIN oauth_client_policies ocp
          ON ocp.client_id = oc.id
         AND ocp.scope_type = 'workspace'
         AND ocp.scope_id = sa.workspace_id
        WHERE sa.workspace_id = $1
        ORDER BY sa.created_at DESC
        "#,
    )
    .bind(access.workspace_id)
    .fetch_all(db)
    .await?;

    Ok(ServiceAccountsResult {
        service_accounts: rows
            .into_iter()
            .map(|row| service_account_view_from_row(row, workspace.organization_id, &member_roles))
            .collect(),
    })
}

pub async fn get_service_account(
    db: &PgPool,
    access: &WorkspaceAccess,
    service_account_id: Uuid,
) -> Result<ServiceAccountView, AppError> {
    let result = list_service_accounts(db, access).await?;
    result
        .service_accounts
        .into_iter()
        .find(|service_account| service_account.principal_id == service_account_id)
        .ok_or_else(|| {
            AppError::not_found(
                "service_account_not_found",
                "The requested service account was not found.",
            )
        })
}

fn service_account_view_from_row(
    row: sqlx::postgres::PgRow,
    organization_id: Option<Uuid>,
    member_roles: &HashMap<Uuid, String>,
) -> ServiceAccountView {
    let principal_id = row.get("principal_id");
    ServiceAccountView {
        principal_id,
        tenant_id: row.get("tenant_id"),
        organization_id,
        workspace_id: row.get("workspace_id"),
        name: row.get("name"),
        description: row.get("description"),
        role: member_roles
            .get(&principal_id)
            .cloned()
            .unwrap_or_else(|| "member".to_string()),
        status: row.get("principal_status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        oauth_clients: oauth_client_from_row(&row).into_iter().collect(),
    }
}

fn oauth_client_from_row(row: &sqlx::postgres::PgRow) -> Option<ServiceAccountClientView> {
    row.get::<Option<Uuid>, _>("oauth_client_uuid")
        .map(|_| service_account_client_view_from_row(row))
}
