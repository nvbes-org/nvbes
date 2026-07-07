use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::{cloud::workspace_port, developer::types::DeveloperServiceAccountSummary},
    http::error::AppError,
};

pub async fn list_service_account_summaries(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperServiceAccountSummary>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          sa.principal_id,
          sa.name,
          sa.description,
          CASE WHEN p.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
          sa.workspace_id,
          COUNT(oc.id)::bigint AS oauth_client_count,
          sa.last_rotated_at
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        LEFT JOIN oauth_clients oc ON oc.client_id = sa.client_id
        WHERE sa.tenant_id = $1
        GROUP BY sa.principal_id, sa.name, sa.description, p.revoked_at, sa.workspace_id, sa.last_rotated_at
        ORDER BY sa.name ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    let mut summaries = Vec::with_capacity(rows.len());
    for row in rows {
        let principal_id: Uuid = row.get("principal_id");
        let workspace_id: Uuid = row.get("workspace_id");
        let role =
            workspace_port::list_workspace_members(Some(tenant_id), workspace_id, principal_id)
                .await?
                .into_iter()
                .find(|member| member.principal_id == principal_id && member.active)
                .map(|member| member.role)
                .unwrap_or_else(|| "viewer".to_string());

        summaries.push(DeveloperServiceAccountSummary {
            principal_id,
            name: row.get("name"),
            description: row.get("description"),
            role,
            status: row.get("status"),
            workspace_id,
            oauth_client_count: row.get("oauth_client_count"),
            last_rotated_at: row.get("last_rotated_at"),
        });
    }

    Ok(summaries)
}

pub async fn current_client_secret_hash(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar(
        "SELECT client_secret_hash FROM oauth_clients WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found"))
}

pub async fn ensure_oauth_client_in_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<(), AppError> {
    current_client_secret_hash(db, tenant_id, client_id)
        .await
        .map(|_| ())
}

pub fn secret_last4(secret: &str) -> &str {
    let split_at = secret.len().saturating_sub(4);
    &secret[split_at..]
}
