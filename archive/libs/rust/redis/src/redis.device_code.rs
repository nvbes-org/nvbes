use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::{RedisError, RedisPool};

const DEVICE_CODE_KEY_PREFIX: &str = "nvbes:identity:oauth-device-code";
const USER_CODE_INDEX_PREFIX: &str = "nvbes:identity:oauth-device-code:user";
const CLIENT_INDEX_PREFIX: &str = "nvbes:identity:oauth-device-code:client";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub client_uuid: Uuid,
    pub client_id: String,
    pub client_name: String,
    pub tenant_id: Uuid,
    pub scope: Vec<String>,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub interval_seconds: i32,
    pub principal_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub denied_at: Option<DateTime<Utc>>,
    pub last_polled_at: Option<DateTime<Utc>>,
}

pub async fn store_device_code(
    pool: &RedisPool,
    device_code: &CachedDeviceCode,
) -> Result<(), RedisError> {
    let ttl_seconds = ttl_seconds(device_code.expires_at);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(device_code)?;

    let _: () = conn
        .set_ex(device_code_key(&device_code.device_code), json, ttl_seconds)
        .await?;
    let _: () = conn
        .set_ex(
            user_code_index_key(&device_code.user_code),
            &device_code.device_code,
            ttl_seconds,
        )
        .await?;
    let _: bool = conn
        .sadd(
            client_index_key(&device_code.client_id),
            &device_code.device_code,
        )
        .await?;
    let _: bool = conn
        .expire(client_index_key(&device_code.client_id), ttl_seconds as i64)
        .await?;

    Ok(())
}

pub async fn get_device_code_by_device_code(
    pool: &RedisPool,
    device_code: &str,
) -> Result<Option<CachedDeviceCode>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn.get(device_code_key(device_code)).await?;
    raw.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

pub async fn get_device_code_by_user_code(
    pool: &RedisPool,
    user_code: &str,
) -> Result<Option<CachedDeviceCode>, RedisError> {
    let mut conn = pool.get().await?;
    let device_code: Option<String> = conn.get(user_code_index_key(user_code)).await?;
    match device_code {
        Some(device_code) => get_device_code_by_device_code(pool, &device_code).await,
        None => Ok(None),
    }
}

pub async fn delete_device_code(
    pool: &RedisPool,
    device_code: &CachedDeviceCode,
) -> Result<(), RedisError> {
    let mut conn = pool.get().await?;
    let _: () = conn
        .del(&[
            device_code_key(&device_code.device_code),
            user_code_index_key(&device_code.user_code),
        ])
        .await?;
    let _: bool = conn
        .srem(
            client_index_key(&device_code.client_id),
            &device_code.device_code,
        )
        .await?;
    Ok(())
}

pub async fn revoke_device_codes_for_client(
    pool: &RedisPool,
    client_id: &str,
) -> Result<u64, RedisError> {
    let mut conn = pool.get().await?;
    let device_codes: Vec<String> = conn.smembers(client_index_key(client_id)).await?;

    let mut revoked = 0;
    for device_code in device_codes {
        if let Some(code) = get_device_code_by_device_code(pool, &device_code).await? {
            delete_device_code(pool, &code).await?;
            revoked += 1;
        }
    }

    let _: () = conn.del(client_index_key(client_id)).await?;
    Ok(revoked)
}

pub fn device_code_key(device_code: &str) -> String {
    format!("{DEVICE_CODE_KEY_PREFIX}:{device_code}")
}

pub fn user_code_index_key(user_code: &str) -> String {
    format!("{USER_CODE_INDEX_PREFIX}:{user_code}")
}

pub fn client_index_key(client_id: &str) -> String {
    format!("{CLIENT_INDEX_PREFIX}:{client_id}")
}

pub fn ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}
