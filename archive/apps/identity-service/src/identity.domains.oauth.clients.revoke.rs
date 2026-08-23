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
    let tenant_id = super::require_oauth_consent_tenant(db, auth).await?;
    let mut tx = db.begin().await?;

    let client = sqlx::query(
        r#"
        SELECT id, client_id
        FROM oauth_clients
        WHERE client_id = $1 AND tenant_id = $2 AND revoked_at IS NULL
          AND EXISTS (
            SELECT 1 FROM oauth_consents
            WHERE oauth_consents.client_id = oauth_clients.id
              AND oauth_consents.principal_id = $3
              AND oauth_consents.tenant_id = $2
              AND oauth_consents.revoked_at IS NULL
          )
        LIMIT 1
        "#,
    )
    .bind(client_id_str)
    .bind(tenant_id)
    .bind(OAuthManagementAuth::user_id(auth))
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
        "UPDATE oauth_clients SET revoked_at = NOW(), updated_at = NOW() WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(client_uuid)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE oauth_consents SET revoked_at = NOW() WHERE client_id = $1 AND tenant_id = $2 AND revoked_at IS NULL",
    )
    .bind(client_uuid)
    .bind(tenant_id)
    .execute(&mut *tx)
    .await?;

    let tokens_revoked =
        nvbes_redis::refresh_token::revoke_all_client_refresh_tokens(redis, client_uuid)
            .await
            .map_err(|error| {
                AppError::internal("oauth_client_refresh_revoke_failed", error.to_string())
            })?;
    let par_revoked =
        nvbes_redis::par::revoke_pushed_authorization_requests_for_client(redis, client_id_str)
            .await
            .map_err(|error| {
                AppError::internal("oauth_client_par_revoke_failed", error.to_string())
            })?;
    let authorization_codes_revoked =
        crate::domains::oauth::authorization_codes::revoke_authorization_codes_for_client(
            redis,
            client_id_str,
        )
        .await?;
    let device_codes_revoked =
        crate::domains::oauth::device_codes::revoke_device_codes_for_client(redis, client_id_str)
            .await?;

    tx.commit().await?;

    tracing::warn!(
        actor_user_id = %OAuthManagementAuth::user_id(auth),
        client_id = %client_id_str,
        client_uuid = %client_uuid,
        tokens_revoked = tokens_revoked,
        par_revoked = par_revoked,
        authorization_codes_revoked = authorization_codes_revoked,
        device_codes_revoked = device_codes_revoked,
        "OAuth client consent revoked"
    );

    Ok(RevokeOAuthClientResult {
        client_id: client_id_str.to_string(),
        tokens_revoked,
    })
}
