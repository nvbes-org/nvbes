use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::connection::{RedisError, RedisPool};

const PUSHED_AUTHORIZATION_REQUEST_KEY_PREFIX: &str = "nvbes:identity:pushed-authorization-request";
const PUSHED_AUTHORIZATION_REQUEST_CLIENT_INDEX_PREFIX: &str =
    "nvbes:identity:pushed-authorization-requests:client";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPushedAuthorizationRequest {
    pub request_uri: String,
    pub client_id: String,
    pub parameters: serde_json::Map<String, serde_json::Value>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

pub async fn store_pushed_authorization_request(
    pool: &RedisPool,
    request: &CachedPushedAuthorizationRequest,
) -> Result<(), RedisError> {
    let ttl_seconds = ttl_seconds(request.expires_at);
    let mut conn = pool.get().await?;
    let json = serde_json::to_string(request)?;
    let _: () = conn
        .set_ex(
            pushed_authorization_request_key(&request.request_uri),
            json,
            ttl_seconds,
        )
        .await?;
    let _: bool = conn
        .sadd(
            pushed_authorization_request_client_index_key(&request.client_id),
            &request.request_uri,
        )
        .await?;
    let _: bool = conn
        .expire(
            pushed_authorization_request_client_index_key(&request.client_id),
            ttl_seconds as i64,
        )
        .await?;
    Ok(())
}

pub async fn get_pushed_authorization_request(
    pool: &RedisPool,
    request_uri: &str,
) -> Result<Option<CachedPushedAuthorizationRequest>, RedisError> {
    let mut conn = pool.get().await?;
    let raw: Option<String> = conn
        .get(pushed_authorization_request_key(request_uri))
        .await?;
    raw.map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(Into::into)
}

pub async fn mark_pushed_authorization_request_used(
    pool: &RedisPool,
    request_uri: &str,
) -> Result<(), RedisError> {
    if let Some(mut request) = get_pushed_authorization_request(pool, request_uri).await? {
        request.used_at = Some(Utc::now());
        store_pushed_authorization_request(pool, &request).await?;
    }
    Ok(())
}

pub async fn revoke_pushed_authorization_requests_for_client(
    pool: &RedisPool,
    client_id: &str,
) -> Result<u64, RedisError> {
    let mut conn = pool.get().await?;
    let request_uris: Vec<String> = conn
        .smembers(pushed_authorization_request_client_index_key(client_id))
        .await?;

    let mut revoked = 0;
    for request_uri in request_uris {
        if let Some(request) = get_pushed_authorization_request(pool, &request_uri).await? {
            if request.client_id == client_id {
                let _: () = conn
                    .del(pushed_authorization_request_key(&request_uri))
                    .await?;
                revoked += 1;
            }
        }
    }

    let _: () = conn
        .del(pushed_authorization_request_client_index_key(client_id))
        .await?;
    Ok(revoked)
}

fn pushed_authorization_request_key(request_uri: &str) -> String {
    format!("{PUSHED_AUTHORIZATION_REQUEST_KEY_PREFIX}:{request_uri}")
}

fn pushed_authorization_request_client_index_key(client_id: &str) -> String {
    format!("{PUSHED_AUTHORIZATION_REQUEST_CLIENT_INDEX_PREFIX}:{client_id}")
}

fn ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    let ttl = (expires_at - Utc::now()).num_seconds().max(1);
    ttl as u64
}
