use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

const REFRESH_TOKEN_KEY_PREFIX: &str = "nvbes:identity:refresh-token";
const REFRESH_TOKEN_SESSION_INDEX_PREFIX: &str = "nvbes:identity:refresh-tokens:session";
const REFRESH_TOKEN_PRINCIPAL_INDEX_PREFIX: &str = "nvbes:identity:refresh-tokens:principal";
const REFRESH_TOKEN_CLIENT_INDEX_PREFIX: &str = "nvbes:identity:refresh-tokens:client";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRefreshToken {
    pub jti: String,
    pub session_id: Uuid,
    pub principal_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub client_id: Option<Uuid>,
    pub scope: String,
    pub authorization_details: Vec<serde_json::Value>,
    pub expires_at: DateTime<Utc>,
    pub rotated_from_jti: Option<String>,
    pub replaced_by_jti: Option<String>,
    pub reuse_detected_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn store_refresh_token(
    pool: &RedisPool,
    token: &CachedRefreshToken,
) -> Result<(), RedisError> {
    let ttl_seconds = ttl_seconds(token.expires_at);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(token)?;

    let _: () = conn
        .set_ex(refresh_token_key(&token.jti), json, ttl_seconds)
        .await?;
    let _: bool = conn
        .sadd(
            refresh_token_session_index_key(&token.session_id),
            &token.jti,
        )
        .await?;
    let _: bool = conn
        .sadd(
            refresh_token_principal_index_key(&token.principal_id),
            &token.jti,
        )
        .await?;
    if let Some(client_id) = token.client_id {
        let _: bool = conn
            .sadd(refresh_token_client_index_key(&client_id), &token.jti)
            .await?;
        let _: bool = conn
            .expire(
                refresh_token_client_index_key(&client_id),
                ttl_seconds as i64,
            )
            .await?;
    }
    let _: bool = conn
        .expire(
            refresh_token_session_index_key(&token.session_id),
            ttl_seconds as i64,
        )
        .await?;
    let _: bool = conn
        .expire(
            refresh_token_principal_index_key(&token.principal_id),
            ttl_seconds as i64,
        )
        .await?;
    Ok(())
}

pub async fn get_refresh_token(
    pool: &RedisPool,
    jti: &str,
) -> Result<Option<CachedRefreshToken>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(refresh_token_key(jti)).await?;
    raw.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

pub async fn set_refresh_token(
    pool: &RedisPool,
    token: &CachedRefreshToken,
) -> Result<(), RedisError> {
    store_refresh_token(pool, token).await
}

pub async fn mark_refresh_token_used(
    pool: &RedisPool,
    jti: &str,
    replaced_by_jti: Option<&str>,
) -> Result<(), RedisError> {
    if let Some(mut token) = get_refresh_token(pool, jti).await? {
        token.last_used_at = Some(Utc::now());
        token.revoked_at = token.revoked_at.or(Some(Utc::now()));
        if let Some(replaced_by_jti) = replaced_by_jti {
            token.replaced_by_jti = Some(replaced_by_jti.to_string());
        }
        set_refresh_token(pool, &token).await?;
    }
    Ok(())
}

pub async fn touch_refresh_token(pool: &RedisPool, jti: &str) -> Result<(), RedisError> {
    if let Some(mut token) = get_refresh_token(pool, jti).await? {
        token.last_used_at = Some(Utc::now());
        set_refresh_token(pool, &token).await?;
    }
    Ok(())
}

pub async fn mark_refresh_token_reuse_detected(
    pool: &RedisPool,
    jti: &str,
) -> Result<(), RedisError> {
    if let Some(mut token) = get_refresh_token(pool, jti).await? {
        token.reuse_detected_at = Some(Utc::now());
        token.revoked_at = Some(Utc::now());
        set_refresh_token(pool, &token).await?;
    }
    Ok(())
}

pub async fn revoke_refresh_token(pool: &RedisPool, jti: &str) -> Result<(), RedisError> {
    if let Some(mut token) = get_refresh_token(pool, jti).await? {
        token.revoked_at = Some(Utc::now());
        set_refresh_token(pool, &token).await?;
    }
    Ok(())
}

pub async fn revoke_refresh_family(
    pool: &RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
    jti: &str,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let session_tokens: Vec<String> = conn
        .smembers(refresh_token_session_index_key(&session_id))
        .await?;
    for token_jti in session_tokens {
        if let Some(mut token) = get_refresh_token(pool, &token_jti).await?
            && token.principal_id == principal_id
            && token.session_id == session_id
        {
            token.revoked_at = Some(Utc::now());
            token.reuse_detected_at = Some(Utc::now());
            token.replaced_by_jti = Some(jti.to_string());
            set_refresh_token(pool, &token).await?;
        }
    }
    Ok(())
}

pub async fn revoke_session_refresh_tokens(
    pool: &RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let session_tokens: Vec<String> = conn
        .smembers(refresh_token_session_index_key(&session_id))
        .await?;
    for token_jti in session_tokens {
        if let Some(mut token) = get_refresh_token(pool, &token_jti).await?
            && token.principal_id == principal_id
            && token.session_id == session_id
        {
            token.revoked_at = Some(Utc::now());
            set_refresh_token(pool, &token).await?;
        }
    }
    Ok(())
}

pub async fn revoke_all_user_refresh_tokens(
    pool: &RedisPool,
    principal_id: Uuid,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let tokens: Vec<String> = conn
        .smembers(refresh_token_principal_index_key(&principal_id))
        .await?;
    for token_jti in tokens {
        if let Some(mut token) = get_refresh_token(pool, &token_jti).await?
            && token.principal_id == principal_id
        {
            token.revoked_at = Some(Utc::now());
            set_refresh_token(pool, &token).await?;
        }
    }
    Ok(())
}

pub async fn revoke_all_user_refresh_tokens_except_session(
    pool: &RedisPool,
    principal_id: Uuid,
    keep_session_id: Uuid,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let tokens: Vec<String> = conn
        .smembers(refresh_token_principal_index_key(&principal_id))
        .await?;
    for token_jti in tokens {
        if let Some(mut token) = get_refresh_token(pool, &token_jti).await?
            && token.principal_id == principal_id
            && token.session_id != keep_session_id
        {
            token.revoked_at = Some(Utc::now());
            set_refresh_token(pool, &token).await?;
        }
    }
    Ok(())
}

pub async fn client_ids_for_session(
    pool: &RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
) -> Result<Vec<Uuid>, RedisError> {
    let mut conn = pool.get().await?;
    let token_ids: Vec<String> = conn
        .smembers(refresh_token_session_index_key(&session_id))
        .await?;
    let mut client_ids = std::collections::HashSet::new();
    for token_id in token_ids {
        if let Some(token) = get_refresh_token(pool, &token_id).await?
            && token.principal_id == principal_id
            && token.session_id == session_id
            && let Some(client_id) = token.client_id
        {
            client_ids.insert(client_id);
        }
    }
    Ok(client_ids.into_iter().collect())
}

pub async fn revoke_all_client_refresh_tokens(
    pool: &RedisPool,
    client_id: Uuid,
) -> Result<u64, RedisError> {
    let mut conn = pool.get().await?;
    let tokens: Vec<String> = conn
        .smembers(refresh_token_client_index_key(&client_id))
        .await?;
    let mut revoked = 0;
    for token_jti in tokens {
        if let Some(mut token) = get_refresh_token(pool, &token_jti).await?
            && token.client_id == Some(client_id)
        {
            token.revoked_at = Some(Utc::now());
            set_refresh_token(pool, &token).await?;
            revoked += 1;
        }
    }
    Ok(revoked)
}

pub async fn is_refresh_token_active(pool: &RedisPool, jti: &str) -> Result<bool, RedisError> {
    match get_refresh_token(pool, jti).await? {
        Some(token) => Ok(token.revoked_at.is_none() && token.reuse_detected_at.is_none()),
        None => Ok(false),
    }
}

pub async fn latest_refresh_token_for_session(
    pool: &RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
) -> Result<Option<CachedRefreshToken>, RedisError> {
    let mut conn = pool.get().await?;
    let tokens: Vec<String> = conn
        .smembers(refresh_token_session_index_key(&session_id))
        .await?;
    let mut latest: Option<CachedRefreshToken> = None;
    for token_jti in tokens {
        if let Some(token) = get_refresh_token(pool, &token_jti).await?
            && token.principal_id == principal_id
            && token.session_id == session_id
        {
            match latest {
                Some(ref current) if current.expires_at >= token.expires_at => {}
                _ => latest = Some(token),
            }
        }
    }
    Ok(latest)
}

fn refresh_token_key(jti: &str) -> String {
    format!("{REFRESH_TOKEN_KEY_PREFIX}:{jti}")
}

fn refresh_token_session_index_key(session_id: &Uuid) -> String {
    format!("{REFRESH_TOKEN_SESSION_INDEX_PREFIX}:{session_id}")
}

fn refresh_token_principal_index_key(principal_id: &Uuid) -> String {
    format!("{REFRESH_TOKEN_PRINCIPAL_INDEX_PREFIX}:{principal_id}")
}

fn refresh_token_client_index_key(client_id: &Uuid) -> String {
    format!("{REFRESH_TOKEN_CLIENT_INDEX_PREFIX}:{client_id}")
}

fn ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}
