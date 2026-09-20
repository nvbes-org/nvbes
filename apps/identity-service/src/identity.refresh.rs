use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use rand::RngCore;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::hash_token;

const REFRESH_TOKEN_TTL_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct RefreshTokenInfo {
    pub token: String,
    pub family_id: Uuid,
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub client_id: String,
    pub scope: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub async fn create_refresh_token(
    db: &PgPool,
    principal_id: Uuid,
    session_id: Uuid,
    client_id: &str,
    scope: &str,
) -> anyhow::Result<RefreshTokenInfo> {
    let family_id = Uuid::new_v4();
    let token = random_token();
    let expires_at = Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS);

    let mut tx = db.begin().await?;

    sqlx::query(
        "INSERT INTO identity_refresh_tokens (id, family_id, principal_id, session_id, token_hash, client_id, scope, expires_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(family_id)
    .bind(principal_id)
    .bind(session_id)
    .bind(hash_token(&token))
    .bind(client_id)
    .bind(scope)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(RefreshTokenInfo {
        token,
        family_id,
        principal_id,
        session_id,
        client_id: client_id.to_string(),
        scope: scope.to_string(),
        expires_at,
    })
}

pub async fn rotate_refresh_token(
    db: &PgPool,
    old_token: &str,
    client_id: &str,
) -> anyhow::Result<RefreshTokenInfo> {
    let old_token_hash = hash_token(old_token);
    let mut tx = db.begin().await?;

    // Get the old token info
    let (family_id, principal_id, session_id, scope): (Uuid, Uuid, Uuid, String) = sqlx::query_as(
        "SELECT family_id, principal_id, session_id, scope 
         FROM identity_refresh_tokens 
         WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > clock_timestamp()
         FOR UPDATE",
    )
    .bind(&old_token_hash)
    .fetch_one(&mut *tx)
    .await?;

    // Revoke the old token
    sqlx::query("UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp() WHERE token_hash = $1")
        .bind(&old_token_hash)
        .execute(&mut *tx)
        .await?;

    // Check for reuse attack - if any other token in the family is already revoked, revoke entire family
    let has_other_revoked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity_refresh_tokens 
         WHERE family_id = $1 AND id != (SELECT id FROM identity_refresh_tokens WHERE token_hash = $2) 
         AND revoked_at IS NOT NULL)",
    )
    .bind(family_id)
    .bind(&old_token_hash)
    .fetch_one(&mut *tx)
    .await?;

    if has_other_revoked {
        // Revoke entire family
        sqlx::query("UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp() WHERE family_id = $1")
            .bind(family_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        anyhow::bail!("Refresh token reuse detected - family revoked");
    }

    // Create new refresh token
    let new_token = random_token();
    let expires_at = Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS);

    sqlx::query(
        "INSERT INTO identity_refresh_tokens (id, family_id, principal_id, session_id, token_hash, client_id, scope, expires_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(family_id)
    .bind(principal_id)
    .bind(session_id)
    .bind(hash_token(&new_token))
    .bind(client_id)
    .bind(&scope)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(RefreshTokenInfo {
        token: new_token,
        family_id,
        principal_id,
        session_id,
        client_id: client_id.to_string(),
        scope,
        expires_at,
    })
}

pub async fn validate_refresh_token(
    db: &PgPool,
    token: &str,
) -> anyhow::Result<(Uuid, String, String)> {
    let token_hash = hash_token(token);

    let (principal_id, client_id, scope): (Uuid, String, String) = sqlx::query_as(
        "SELECT principal_id, client_id, scope 
         FROM identity_refresh_tokens 
         WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > clock_timestamp()",
    )
    .bind(&token_hash)
    .fetch_one(db)
    .await?;

    Ok((principal_id, client_id, scope))
}

pub async fn revoke_refresh_token(db: &PgPool, token: &str) -> anyhow::Result<()> {
    let token_hash = hash_token(token);

    sqlx::query("UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp() WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn revoke_refresh_token_family(db: &PgPool, family_id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp() WHERE family_id = $1")
        .bind(family_id)
        .execute(db)
        .await?;

    Ok(())
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}