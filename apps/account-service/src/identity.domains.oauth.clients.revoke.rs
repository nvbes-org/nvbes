use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::oauth::{logic::OAuthManagementAuth, service::types::RevokeOAuthClientResult};
use crate::http::error::AppError;

/// Revoke an OAuth client.
pub async fn revoke_client(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    client_id_str: &str,
) -> Result<RevokeOAuthClientResult, AppError> {
    let tenant_id = super::require_oauth_management_tenant(db, auth).await?;
    let mut tx = db.begin().await?;

    let client = sqlx::query(
        r#"
        SELECT id, client_id
        FROM oauth_clients
        WHERE client_id = $1
          AND tenant_id = $2
          AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(client_id_str)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?;

    let client = client.ok_or_else(|| {
        AppError::not_found(
            "client_not_found",
            "OAuth client not found or already revoked.",
        )
    })?;

    let client_uuid: Uuid = client.get("id");

    sqlx::query(
        r#"
        UPDATE oauth_clients
        SET revoked_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(client_uuid)
    .execute(&mut *tx)
    .await?;

    let tokens_revoked =
        nvbes_redis::refresh_token::revoke_all_client_refresh_tokens(redis, client_uuid)
            .await
            .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;
    let par_revoked =
        nvbes_redis::par::revoke_pushed_authorization_requests_for_client(redis, client_id_str)
            .await
            .map_err(|err| {
                AppError::internal(
                    "pushed_authorization_request_revoke_failed",
                    err.to_string(),
                )
            })?;

    crate::domains::oauth::authorization_codes::revoke_authorization_codes_for_client(
        redis,
        client_id_str,
    )
    .await?;
    crate::domains::oauth::device_codes::revoke_device_codes_for_client(redis, client_id_str)
        .await?;

    tx.commit().await?;

    tracing::warn!(
        actor_user_id = %OAuthManagementAuth::user_id(auth),
        client_id = %client_id_str,
        client_uuid = %client_uuid,
        tokens_revoked = tokens_revoked,
        par_revoked = par_revoked,
        "OAuth client revoked"
    );

    Ok(RevokeOAuthClientResult {
        client_id: client_id_str.to_string(),
        tokens_revoked,
    })
}
