use axum::http::{HeaderMap, Method, Uri};
use sha2::{Digest, Sha256};
const IDEMPOTENCY_KEY_MAX_LEN: usize = 255;
const DEFAULT_TTL_HOURS: i32 = 24;

#[derive(Debug, Clone)]
pub struct IdempotencyScope(pub String);

impl IdempotencyScope {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdempotencyScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub fn validate_key(key: &str) -> Result<&str, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("Idempotency-Key must not be empty".to_string());
    }
    if trimmed.len() > IDEMPOTENCY_KEY_MAX_LEN {
        return Err(format!(
            "Idempotency-Key must not exceed {IDEMPOTENCY_KEY_MAX_LEN} characters"
        ));
    }
    Ok(trimmed)
}

pub fn derive_scope(headers: &HeaderMap, method: &Method, uri: &Uri) -> IdempotencyScope {
    let mut hasher = Sha256::new();

    if let Some(auth) = headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        hasher.update(b"auth:");
        hasher.update(auth.as_bytes());
    } else if let Some(ip) = headers
        .get("x-nvbes-client-ip")
        .and_then(|v| v.to_str().ok())
    {
        hasher.update(b"ip:");
        hasher.update(ip.as_bytes());
    } else {
        hasher.update(b"anon:");
    }

    hasher.update(b":");
    hasher.update(method.as_str().as_bytes());
    hasher.update(b":");
    hasher.update(uri.path().as_bytes());

    let hash = data_encoding::HEXLOWER.encode(&hasher.finalize());
    IdempotencyScope(hash)
}

pub fn hash_request_body(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    data_encoding::HEXLOWER.encode(&hasher.finalize())
}

pub fn build_request_signature(method: &Method, uri: &Uri, body: &[u8]) -> String {
    let body_hash = hash_request_body(body);
    format!("{}:{}:{}", method.as_str(), uri.path(), body_hash)
}

pub struct StoredIdempotencyResponse {
    pub response_status: i32,
    pub response_body: Vec<u8>,
    pub request_hash: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredIdempotencyResponseWire {
    response_status: i32,
    response_body_base64: String,
    request_hash: String,
}

impl From<&StoredIdempotencyResponse> for StoredIdempotencyResponseWire {
    fn from(s: &StoredIdempotencyResponse) -> Self {
        use base64::Engine;
        Self {
            response_status: s.response_status,
            response_body_base64: base64::engine::general_purpose::STANDARD
                .encode(&s.response_body),
            request_hash: s.request_hash.clone(),
        }
    }
}

impl TryFrom<StoredIdempotencyResponseWire> for StoredIdempotencyResponse {
    type Error = String;

    fn try_from(w: StoredIdempotencyResponseWire) -> Result<Self, Self::Error> {
        use base64::Engine;
        let response_body = base64::engine::general_purpose::STANDARD
            .decode(&w.response_body_base64)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            response_status: w.response_status,
            response_body,
            request_hash: w.request_hash,
        })
    }
}

pub async fn fetch_idempotency_response(
    redis: &nvbes_redis::RedisPool,
    key: &str,
    scope: &IdempotencyScope,
) -> Result<Option<StoredIdempotencyResponse>, sqlx::Error> {
    let redis_key = format!("nvbes:idem:{}:{}", scope.as_str(), key);
    let wire: Option<StoredIdempotencyResponseWire> =
        nvbes_redis::cache::cache_get_json(redis, &redis_key)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Redis error: {e}")))?;

    match wire {
        Some(w) => {
            let stored = StoredIdempotencyResponse::try_from(w).map_err(sqlx::Error::Protocol)?;
            Ok(Some(stored))
        }
        None => Ok(None),
    }
}

pub async fn insert_idempotency_response(
    redis: &nvbes_redis::RedisPool,
    key: &str,
    scope: &IdempotencyScope,
    request_hash: &str,
    response_status: i32,
    response_body: &[u8],
) -> Result<(), sqlx::Error> {
    let redis_key = format!("nvbes:idem:{}:{}", scope.as_str(), key);
    let stored = StoredIdempotencyResponse {
        response_status,
        response_body: response_body.to_vec(),
        request_hash: request_hash.to_string(),
    };
    let wire = StoredIdempotencyResponseWire::from(&stored);
    let ttl = DEFAULT_TTL_HOURS as u64 * 3600;
    nvbes_redis::cache::cache_set_json(redis, &redis_key, &wire, ttl)
        .await
        .map_err(|e| sqlx::Error::Protocol(format!("Redis error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_redis_pool() -> nvbes_redis::RedisPool {
        let config = nvbes_redis::RedisConfig::from_env();
        nvbes_redis::connection::create_pool(&config)
            .await
            .expect("redis pool")
    }

    #[tokio::test]
    async fn idempotency_round_trip_uses_redis() {
        let redis = test_redis_pool().await;
        let key = format!("idem-key-{}", uuid::Uuid::new_v4());
        let scope = IdempotencyScope(format!("scope-{}", uuid::Uuid::new_v4()));
        let request_hash = "request-hash-123";
        let response_body = br#"{"ok":true}"#;

        insert_idempotency_response(&redis, &key, &scope, request_hash, 201, response_body)
            .await
            .expect("insert should succeed");

        let stored = fetch_idempotency_response(&redis, &key, &scope)
            .await
            .expect("fetch should succeed")
            .expect("stored response should exist");

        assert_eq!(stored.response_status, 201);
        assert_eq!(stored.response_body, response_body);
        assert_eq!(stored.request_hash, request_hash);
    }
}
