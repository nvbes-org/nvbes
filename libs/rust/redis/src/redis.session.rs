use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::connection::{RedisError, RedisPool};

const SESSION_KEY_PREFIX: &str = "nvbes:identity:session";
const USER_SESSION_INDEX_PREFIX: &str = "nvbes:identity:user-sessions";
const REVOKED_SESSION_TTL_SECONDS: u64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub session_id: String,
    pub principal_id: String,
    #[serde(default)]
    pub browser_session_token_hash: Option<String>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_region: Option<String>,
    pub client_id: Option<String>,
    pub acr: Option<String>,
    pub amr: Vec<String>,
    pub auth_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    #[serde(default)]
    pub idle_timeout_seconds: Option<i64>,
    #[serde(default)]
    pub idle_expires_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub step_up_verified_at: Option<DateTime<Utc>>,
    pub step_up_expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
    #[serde(default)]
    pub geo_country_code: Option<String>,
    pub user_agent: Option<String>,
    #[serde(default)]
    pub accept_language: Option<String>,
    #[serde(default)]
    pub accept: Option<String>,
    #[serde(default)]
    pub accept_encoding: Option<String>,
    #[serde(default)]
    pub sec_fetch_site: Option<String>,
    #[serde(default)]
    pub sec_fetch_mode: Option<String>,
    #[serde(default)]
    pub sec_fetch_dest: Option<String>,
    #[serde(default)]
    pub sec_ch_ua: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_arch: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_bitness: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_full_version: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_full_version_list: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_model: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_wow64: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_form_factors: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_platform: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_platform_version: Option<String>,
    #[serde(default)]
    pub sec_ch_ua_mobile: Option<String>,
    #[serde(default)]
    pub cookie_theft_risk_score: Option<f64>,
    #[serde(default)]
    pub cookie_theft_detected_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub account_device_id: Option<String>,
    #[serde(default)]
    pub device_trust_level: Option<String>,
    #[serde(default)]
    pub device_trust_score: Option<i16>,
    #[serde(default)]
    pub risk_score: Option<f64>,
    #[serde(default)]
    pub risk_decision: Option<String>,
    #[serde(default)]
    pub activity_window_started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub activity_request_count: u32,
    #[serde(default)]
    pub last_activity_risk_event_at: Option<DateTime<Utc>>,
}

pub async fn get_session(
    pool: &RedisPool,
    session_id: &str,
) -> Result<Option<CachedSession>, RedisError> {
    let key = session_key(session_id);
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(&key).await?;

    raw.map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(Into::into)
}

pub async fn set_session(
    pool: &RedisPool,
    session: &CachedSession,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let key = session_key(&session.session_id);
    let index_key = user_session_index_key(&session.principal_id);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(session)?;

    let _: () = conn.set_ex(&key, json, ttl_seconds).await?;
    let _: bool = conn.sadd(&index_key, &session.session_id).await?;
    let _: bool = conn.expire(&index_key, ttl_seconds as i64).await?;
    Ok(())
}

pub async fn delete_session(
    pool: &RedisPool,
    principal_id: &str,
    session_id: &str,
) -> Result<(), RedisError> {
    add_revoked_session(pool, session_id, REVOKED_SESSION_TTL_SECONDS).await?;
    let key = session_key(session_id);
    let index_key = user_session_index_key(principal_id);
    let mut conn = pool.get().await?;
    let _: () = conn.del(&key).await?;
    let _: bool = conn.srem(&index_key, session_id).await?;
    let _ = crate::pubsub::publish_event(pool, "session:revoked", session_id).await;
    Ok(())
}

pub async fn touch_session(
    pool: &RedisPool,
    session_id: &str,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let key = session_key(session_id);
    let mut conn = pool.get().await?;
    let _: bool = conn.expire(&key, ttl_seconds as i64).await?;
    Ok(())
}

pub async fn update_session_workspace(
    pool: &RedisPool,
    session_id: &str,
    tenant_id: Option<String>,
    organization_id: Option<String>,
    workspace_id: Option<String>,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    if let Some(mut session) = get_session(pool, session_id).await? {
        session.tenant_id = tenant_id;
        session.organization_id = organization_id;
        session.workspace_id = workspace_id;
        set_session(pool, &session, ttl_seconds).await?;
    }
    Ok(())
}

pub async fn update_session_step_up(
    pool: &RedisPool,
    session_id: &str,
    acr: Option<String>,
    amr: Vec<String>,
    auth_time: Option<DateTime<Utc>>,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    if let Some(mut session) = get_session(pool, session_id).await? {
        session.acr = acr;
        session.amr = amr;
        session.auth_time = auth_time;
        set_session(pool, &session, ttl_seconds).await?;
    }
    Ok(())
}

pub async fn clear_user_sessions(pool: &RedisPool, principal_id: &str) -> Result<(), RedisError> {
    let key = user_session_index_key(principal_id);
    let mut conn = pool.get().await?;
    let session_ids: Vec<String> = conn.smembers(&key).await?;

    for session_id in session_ids {
        add_revoked_session(pool, &session_id, REVOKED_SESSION_TTL_SECONDS).await?;
        let _: () = conn.del(session_key(&session_id)).await?;
        let _ = crate::pubsub::publish_event(pool, "session:revoked", &session_id).await;
    }

    let _: () = conn.del(&key).await?;
    Ok(())
}

pub async fn clear_user_sessions_except(
    pool: &RedisPool,
    principal_id: &str,
    keep_session_id: &str,
) -> Result<(), RedisError> {
    let key = user_session_index_key(principal_id);
    let mut conn = pool.get().await?;
    let session_ids: Vec<String> = conn.smembers(&key).await?;

    for session_id in session_ids {
        if session_id != keep_session_id {
            add_revoked_session(pool, &session_id, REVOKED_SESSION_TTL_SECONDS).await?;
            let _: () = conn.del(session_key(&session_id)).await?;
            let _: bool = conn.srem(&key, &session_id).await?;
            let _ = crate::pubsub::publish_event(pool, "session:revoked", &session_id).await;
        }
    }

    Ok(())
}

pub async fn list_user_sessions(
    pool: &RedisPool,
    principal_id: &str,
) -> Result<Vec<String>, RedisError> {
    let key = user_session_index_key(principal_id);
    let mut conn = pool.get().await?;
    conn.smembers(&key).await.map_err(Into::into)
}

pub fn session_key(session_id: &str) -> String {
    format!("{SESSION_KEY_PREFIX}:{session_id}")
}

pub fn user_session_index_key(principal_id: &str) -> String {
    format!("{USER_SESSION_INDEX_PREFIX}:{principal_id}")
}

pub async fn add_revoked_session(
    pool: &RedisPool,
    session_id: &str,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let key = "nvbes:identity:sessions:revoked";
    let mut conn = pool.get().await?;
    let _: bool = conn.sadd(key, session_id).await?;
    let _: bool = conn.expire(key, ttl_seconds as i64).await?;
    Ok(())
}

pub async fn is_session_revoked(pool: &RedisPool, session_id: &str) -> Result<bool, RedisError> {
    let key = "nvbes:identity:sessions:revoked";
    let mut conn = pool.get().await?;
    let result: bool = conn.sismember(key, session_id).await?;
    Ok(result)
}
