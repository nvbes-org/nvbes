use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::connection::{RedisError, RedisPool};

const AUTH_STATE_KEY_PREFIX: &str = "nvbes:identity:auth-state";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAuthState {
    pub id: String,
    pub principal_id: Option<String>,
    pub email: String,
    pub next_step: String,
    pub device_fingerprint: Option<serde_json::Value>,
    #[serde(default)]
    pub completed_methods: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

pub async fn set_auth_state(
    pool: &RedisPool,
    state: &CachedAuthState,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let key = auth_state_key(&state.id);
    let json = serde_json::to_string(state)?;
    let mut conn = pool.get().await?;
    let _: () = conn.set_ex(&key, json, ttl_seconds).await?;
    Ok(())
}

pub async fn get_auth_state(
    pool: &RedisPool,
    id: &str,
) -> Result<Option<CachedAuthState>, RedisError> {
    let key = auth_state_key(id);
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(&key).await?;
    raw.map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(Into::into)
}

pub async fn delete_auth_state(pool: &RedisPool, id: &str) -> Result<(), RedisError> {
    let key = auth_state_key(id);
    let mut conn = pool.get().await?;
    let _: () = conn.del(&key).await?;
    Ok(())
}

pub fn auth_state_key(id: &str) -> String {
    format!("{AUTH_STATE_KEY_PREFIX}:{id}")
}
