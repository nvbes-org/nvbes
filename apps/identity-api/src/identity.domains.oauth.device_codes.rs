use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::http::error::AppError;

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
    redis: &nvbes_redis::RedisPool,
    device_code: &CachedDeviceCode,
) -> Result<(), AppError> {
    let ttl_seconds = ttl_seconds(device_code.expires_at);
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_set_json(
            &device_code_key(&device_code.device_code),
            device_code,
            ttl_seconds,
        )
        .await
        .map_err(|err| AppError::internal("device_code_store_failed", format!("{}", err)))?;
    client
        .set_with_ttl(
            &user_code_index_key(&device_code.user_code),
            &device_code.device_code,
            ttl_seconds,
        )
        .await
        .map_err(|err| AppError::internal("device_code_store_failed", format!("{}", err)))?;
    client
        .sadd(
            &client_index_key(&device_code.client_id),
            &device_code.device_code,
        )
        .await
        .map_err(|err| AppError::internal("device_code_store_failed", format!("{}", err)))?;
    client
        .expire(
            &client_index_key(&device_code.client_id),
            ttl_seconds as i64,
        )
        .await
        .map_err(|err| AppError::internal("device_code_store_failed", format!("{}", err)))?;
    Ok(())
}

pub async fn get_device_code_by_device_code(
    redis: &nvbes_redis::RedisPool,
    device_code: &str,
) -> Result<Option<CachedDeviceCode>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_get_json(&device_code_key(device_code))
        .await
        .map_err(|err| AppError::internal("device_code_load_failed", format!("{}", err)))
}

pub async fn get_device_code_by_user_code(
    redis: &nvbes_redis::RedisPool,
    user_code: &str,
) -> Result<Option<CachedDeviceCode>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let device_code = client
        .get(&user_code_index_key(user_code))
        .await
        .map_err(|err| AppError::internal("device_code_load_failed", format!("{}", err)))?;

    let Some(device_code) = device_code else {
        return Ok(None);
    };

    get_device_code_by_device_code(redis, &device_code).await
}

pub async fn save_device_code(
    redis: &nvbes_redis::RedisPool,
    device_code: &CachedDeviceCode,
) -> Result<(), AppError> {
    store_device_code(redis, device_code).await
}

pub async fn delete_device_code(
    redis: &nvbes_redis::RedisPool,
    device_code: &CachedDeviceCode,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .del(&[
            &device_code_key(&device_code.device_code),
            &user_code_index_key(&device_code.user_code),
        ])
        .await
        .map_err(|err| AppError::internal("device_code_delete_failed", format!("{}", err)))?;
    client
        .srem(
            &client_index_key(&device_code.client_id),
            &device_code.device_code,
        )
        .await
        .map_err(|err| AppError::internal("device_code_delete_failed", format!("{}", err)))?;
    Ok(())
}

pub async fn revoke_device_codes_for_client(
    redis: &nvbes_redis::RedisPool,
    client_id: &str,
) -> Result<u64, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let device_codes = client
        .smembers(&client_index_key(client_id))
        .await
        .map_err(|err| AppError::internal("device_code_revoke_failed", format!("{}", err)))?;

    let mut revoked = 0;
    for device_code in device_codes {
        if let Some(code) = get_device_code_by_device_code(redis, &device_code).await? {
            let client = nvbes_redis::RedisClient::new(redis.clone());
            client
                .del(&[
                    &device_code_key(&code.device_code),
                    &user_code_index_key(&code.user_code),
                ])
                .await
                .map_err(|err| {
                    AppError::internal("device_code_revoke_failed", format!("{}", err))
                })?;
            client
                .srem(&client_index_key(&code.client_id), &code.device_code)
                .await
                .map_err(|err| {
                    AppError::internal("device_code_revoke_failed", format!("{}", err))
                })?;
            revoked += 1;
        }
    }

    client
        .del_key(&client_index_key(client_id))
        .await
        .map_err(|err| AppError::internal("device_code_revoke_failed", format!("{}", err)))?;

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

pub fn is_expired(expires_at: DateTime<Utc>) -> bool {
    expires_at < Utc::now()
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use uuid::Uuid;

    use super::{
        CachedDeviceCode, delete_device_code, get_device_code_by_device_code,
        get_device_code_by_user_code, save_device_code,
    };

    #[tokio::test]
    async fn stores_and_loads_device_code_via_user_code() {
        let redis = crate::test_support::test_redis_pool().await;
        let code = CachedDeviceCode {
            device_code: format!("gxdc_{}", Uuid::new_v4().simple()),
            user_code: "ABCD-EFGH".to_string(),
            client_uuid: Uuid::new_v4(),
            client_id: format!("gxoc_{}", Uuid::new_v4().simple()),
            client_name: "Example App".to_string(),
            tenant_id: Uuid::new_v4(),
            scope: vec!["openid".to_string()],
            audience: None,
            resource_indicators: Vec::new(),
            verification_uri: "https://example.com/activate".to_string(),
            verification_uri_complete: Some(
                "https://example.com/activate?user_code=ABCD-EFGH".to_string(),
            ),
            expires_at: chrono::Utc::now() + Duration::minutes(5),
            interval_seconds: 5,
            principal_id: None,
            session_id: None,
            organization_id: None,
            workspace_id: None,
            approved_at: None,
            denied_at: None,
            last_polled_at: None,
        };

        save_device_code(&redis, &code)
            .await
            .expect("store device code");
        let loaded = get_device_code_by_user_code(&redis, &code.user_code)
            .await
            .expect("load by user code")
            .expect("device code exists");
        assert_eq!(loaded.device_code, code.device_code);
        assert_eq!(loaded.client_name, code.client_name);

        let direct = get_device_code_by_device_code(&redis, &code.device_code)
            .await
            .expect("load by device code")
            .expect("device code exists");
        assert_eq!(direct.user_code, code.user_code);

        delete_device_code(&redis, &direct)
            .await
            .expect("delete device code");
        assert!(
            get_device_code_by_user_code(&redis, &code.user_code)
                .await
                .expect("load after delete")
                .is_none()
        );
    }
}
