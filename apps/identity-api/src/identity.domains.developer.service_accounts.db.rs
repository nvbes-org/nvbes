use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::developer::types::{DeveloperSecretVersionSummary, DeveloperServiceAccountSummary},
    http::error::AppError,
};

pub async fn list_service_account_summaries(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperServiceAccountSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          sa.principal_id,
          sa.name,
          sa.description,
          COALESCE(wm.role::text, 'viewer') AS role,
          CASE WHEN p.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
          sa.workspace_id,
          COUNT(oc.id)::bigint AS oauth_client_count,
          sa.last_rotated_at
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        LEFT JOIN workspace_memberships wm
          ON wm.workspace_id = sa.workspace_id
         AND wm.principal_id = sa.principal_id
        LEFT JOIN oauth_clients oc ON oc.client_id = sa.client_id
        WHERE sa.tenant_id = $1
        GROUP BY sa.principal_id, sa.name, sa.description, wm.role, p.revoked_at, sa.workspace_id, sa.last_rotated_at
        ORDER BY sa.name ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn list_secret_version_summaries(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<Vec<DeveloperSecretVersionSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT id, client_id, status::text AS status, secret_last4, created_at, expires_at, revoked_at
        FROM developer_client_secret_versions
        WHERE tenant_id = $1
          AND client_id = $2
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
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

pub async fn ensure_previous_overlap_version(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    client_id: &str,
    current_hash: &str,
    overlap_ends_at: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        r#"
        UPDATE developer_client_secret_versions
        SET status = 'overlap',
            expires_at = $3
        WHERE tenant_id = $1
          AND client_id = $2
          AND status = 'active'
          AND revoked_at IS NULL
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(overlap_ends_at)
    .fetch_optional(&mut **tx)
    .await?
    {
        return Ok(id);
    }

    sqlx::query_scalar(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          expires_at
        )
        VALUES ($1, $2, 'overlap', $3, 'unknown', $4)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(current_hash)
    .bind(overlap_ends_at)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::from)
}

pub fn secret_last4(secret: &str) -> &str {
    let split_at = secret.len().saturating_sub(4);
    &secret[split_at..]
}
