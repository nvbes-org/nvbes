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
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub scope: String,
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
        principal_id,
        session_id,
        scope: scope.to_string(),
    })
}

pub async fn rotate_refresh_token(
    db: &PgPool,
    old_token: &str,
    client_id: &str,
) -> anyhow::Result<RefreshTokenInfo> {
    let old_token_hash = hash_token(old_token);
    let mut tx = db.begin().await?;

    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            Uuid,
            String,
            Option<chrono::DateTime<Utc>>,
            chrono::DateTime<Utc>,
            String,
        ),
    >(
        "SELECT id, family_id, principal_id, session_id, scope, revoked_at, expires_at, client_id 
         FROM identity_refresh_tokens 
         WHERE token_hash = $1 
         FOR UPDATE",
    )
    .bind(&old_token_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let (id, family_id, principal_id, session_id, scope, revoked_at, expires_at, stored_client_id) =
        match row {
            Some(r) => r,
            None => anyhow::bail!("Invalid refresh token"),
        };

    if stored_client_id != client_id {
        anyhow::bail!("Client mismatch for refresh token");
    }

    if revoked_at.is_some() {
        sqlx::query(
            "UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp() WHERE family_id = $1 AND revoked_at IS NULL",
        )
        .bind(family_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        anyhow::bail!("Refresh token reuse detected - family revoked");
    }

    if expires_at <= Utc::now() {
        anyhow::bail!("Refresh token expired");
    }

    sqlx::query(
        "UPDATE identity_refresh_tokens SET revoked_at = clock_timestamp(), last_used_at = clock_timestamp() WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    let new_token = random_token();
    let new_expires_at = Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS);

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
    .bind(new_expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(RefreshTokenInfo {
        token: new_token,
        principal_id,
        session_id,
        scope,
    })
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
