use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::oauth::{logic::OAuthManagementAuth, service::types::RevokeOAuthClientResult};
use crate::http::error::AppError;

/// Revoke an OAuth client.
pub async fn revoke_client(
    db: &PgPool,
    _redis: &nvbes_redis::RedisPool,
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
        "UPDATE oauth_consents SET revoked_at = NOW() WHERE client_id = $1 AND principal_id = $2 AND tenant_id = $3 AND revoked_at IS NULL",
    )
    .bind(client_uuid)
    .bind(OAuthManagementAuth::user_id(auth))
    .bind(tenant_id)
    .execute(&mut *tx)
    .await?;
    let tokens_revoked = 0;
    let par_revoked = 0;

    tx.commit().await?;

    tracing::warn!(
        actor_user_id = %OAuthManagementAuth::user_id(auth),
        client_id = %client_id_str,
        client_uuid = %client_uuid,
        tokens_revoked = tokens_revoked,
        par_revoked = par_revoked,
        "OAuth client consent revoked"
    );

    Ok(RevokeOAuthClientResult {
        client_id: client_id_str.to_string(),
        tokens_revoked,
    })
}
