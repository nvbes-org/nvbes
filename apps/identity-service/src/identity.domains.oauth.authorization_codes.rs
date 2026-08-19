use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{domains::oauth::rar::AuthorizationDetails, http::error::AppError};

const CODE_KEY_PREFIX: &str = "nvbes:identity:authorization-code";
const CLIENT_INDEX_PREFIX: &str = "nvbes:identity:authorization-codes:client";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub client_uuid: Uuid,
    pub user_id: Uuid,
    pub client_session_id: Option<Uuid>,
    pub redirect_uri: String,
    pub nonce: Option<String>,
    pub scope: String,
    pub audience: Option<String>,
    pub resource_indicators: Vec<String>,
    pub authorization_details: AuthorizationDetails,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub dpop_jkt: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

pub async fn store_authorization_code(
    redis: &nvbes_redis::RedisPool,
    code: &CachedAuthorizationCode,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let ttl_seconds = code_ttl_seconds(code.expires_at);

    client
        .cache_set_json(&code_key(&code.code), code, ttl_seconds)
        .await
        .map_err(|err| AppError::internal("authorization_code_store_failed", format!("{}", err)))?;
    client
        .sadd(&client_index_key(&code.client_id), &code.code)
        .await
        .map_err(|err| AppError::internal("authorization_code_store_failed", format!("{}", err)))?;
    client
        .expire(&client_index_key(&code.client_id), ttl_seconds as i64)
        .await
        .map_err(|err| AppError::internal("authorization_code_store_failed", format!("{}", err)))?;

    Ok(())
}

pub async fn get_authorization_code(
    redis: &nvbes_redis::RedisPool,
    code: &str,
) -> Result<Option<CachedAuthorizationCode>, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_get_json(&code_key(code))
        .await
        .map_err(|err| AppError::internal("authorization_code_load_failed", format!("{}", err)))
}

pub async fn mark_authorization_code_consumed(
    redis: &nvbes_redis::RedisPool,
    code: &str,
) -> Result<(), AppError> {
    let Some(mut authorization_code) = get_authorization_code(redis, code).await? else {
        return Err(AppError::bad_request("invalid_grant", "Invalid code."));
    };

    if authorization_code.expires_at <= Utc::now() || authorization_code.consumed_at.is_some() {
        return Err(AppError::bad_request(
            "invalid_grant",
            "Code expired or already used.",
        ));
    }

    authorization_code.consumed_at = Some(Utc::now());
    let ttl_seconds = code_ttl_seconds(authorization_code.expires_at);
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client
        .cache_set_json(&code_key(code), &authorization_code, ttl_seconds)
        .await
        .map_err(|err| AppError::internal("authorization_code_store_failed", format!("{}", err)))
}

pub async fn delete_authorization_code(
    redis: &nvbes_redis::RedisPool,
    code: &CachedAuthorizationCode,
) -> Result<(), AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    client.del_key(&code_key(&code.code)).await.map_err(|err| {
        AppError::internal("authorization_code_delete_failed", format!("{}", err))
    })?;
    client
        .srem(&client_index_key(&code.client_id), &code.code)
        .await
        .map_err(|err| {
            AppError::internal("authorization_code_delete_failed", format!("{}", err))
        })?;
    Ok(())
}

pub async fn revoke_authorization_codes_for_client(
    redis: &nvbes_redis::RedisPool,
    client_id: &str,
) -> Result<u64, AppError> {
    let client = nvbes_redis::RedisClient::new(redis.clone());
    let code_ids = client
        .smembers(&client_index_key(client_id))
        .await
        .map_err(|err| {
            AppError::internal("authorization_code_revoke_failed", format!("{}", err))
        })?;

    let mut revoked = 0;
    for code in code_ids {
        client.del_key(&code_key(&code)).await.map_err(|err| {
            AppError::internal("authorization_code_revoke_failed", format!("{}", err))
        })?;
        revoked += 1;
    }

    client
        .del_key(&client_index_key(client_id))
        .await
        .map_err(|err| {
            AppError::internal("authorization_code_revoke_failed", format!("{}", err))
        })?;

    Ok(revoked)
}

fn code_key(code: &str) -> String {
    format!("{CODE_KEY_PREFIX}:{code}")
}

fn client_index_key(client_id: &str) -> String {
    format!("{CLIENT_INDEX_PREFIX}:{client_id}")
}

fn code_ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use serde_json::json;
    use uuid::Uuid;

    use super::{
        CachedAuthorizationCode, delete_authorization_code, get_authorization_code,
        mark_authorization_code_consumed, revoke_authorization_codes_for_client,
        store_authorization_code,
    };

    #[tokio::test]
    async fn stores_and_consumes_authorization_code() {
        let Some(redis) = crate::test_support::test_redis_pool().await else {
            eprintln!("skipping test: redis not available");
            return;
        };
        let code = CachedAuthorizationCode {
            code: format!("gxac_{}", Uuid::new_v4().simple()),
            client_id: format!("gxoc_{}", Uuid::new_v4().simple()),
            client_uuid: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            client_session_id: Some(Uuid::new_v4()),
            redirect_uri: "https://example.com/callback".to_string(),
            scope: "openid profile".to_string(),
            nonce: Some("nonce".to_string()),
            audience: Some("https://api.example.com".to_string()),
            resource_indicators: vec!["https://resource.example.com".to_string()],
            authorization_details: vec![json!({"type": "example"})],
            code_challenge: Some("challenge".to_string()),
            code_challenge_method: Some("S256".to_string()),
            dpop_jkt: Some("test-jkt".to_string()),
            tenant_id: Some(Uuid::new_v4()),
            organization_id: Some(Uuid::new_v4()),
            workspace_id: Some(Uuid::new_v4()),
            expires_at: chrono::Utc::now() + Duration::minutes(10),
            consumed_at: None,
        };

        store_authorization_code(&redis, &code)
            .await
            .expect("store code");
        let loaded = get_authorization_code(&redis, &code.code)
            .await
            .expect("load code")
            .expect("code stored");

        assert_eq!(loaded.code, code.code);
        assert_eq!(loaded.client_id, code.client_id);

        mark_authorization_code_consumed(&redis, &code.code)
            .await
            .expect("mark consumed");
        let consumed = get_authorization_code(&redis, &code.code)
            .await
            .expect("reload code")
            .expect("code still present");
        assert!(consumed.consumed_at.is_some());

        delete_authorization_code(&redis, &consumed)
            .await
            .expect("delete code");
        assert!(
            get_authorization_code(&redis, &code.code)
                .await
                .expect("final load")
                .is_none()
        );
    }

    #[tokio::test]
    async fn revokes_authorization_codes_by_client() {
        let Some(redis) = crate::test_support::test_redis_pool().await else {
            eprintln!("skipping test: redis not available");
            return;
        };
        let client_id = format!("gxoc_{}", Uuid::new_v4().simple());

        for _ in 0..2 {
            let code = CachedAuthorizationCode {
                code: format!("gxac_{}", Uuid::new_v4().simple()),
                client_id: client_id.clone(),
                client_uuid: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                client_session_id: None,
                redirect_uri: "https://example.com/callback".to_string(),
                scope: "openid".to_string(),
                nonce: None,
                audience: None,
                resource_indicators: Vec::new(),
                authorization_details: Vec::new(),
                code_challenge: None,
                code_challenge_method: None,
                dpop_jkt: None,
                tenant_id: None,
                organization_id: None,
                workspace_id: None,
                expires_at: chrono::Utc::now() + Duration::minutes(10),
                consumed_at: None,
            };
            store_authorization_code(&redis, &code)
                .await
                .expect("store code");
        }

        let revoked = revoke_authorization_codes_for_client(&redis, &client_id)
            .await
            .expect("revoke codes");
        assert_eq!(revoked, 2);
    }
}
