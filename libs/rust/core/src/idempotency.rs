use axum::http::{HeaderMap, Method, Uri};
use sha2::{Digest, Sha256};
const IDEMPOTENCY_KEY_MAX_LEN: usize = 255;
const DEFAULT_TTL_HOURS: i32 = 24;
const IN_FLIGHT_TTL_SECONDS: u64 = 120;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyClaim {
    Acquired { release_token: String },
    InProgress { request_hash: String },
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
    let redis_key = idempotency_response_key(key, scope);
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
    let redis_key = idempotency_response_key(key, scope);
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

pub async fn claim_idempotency_request(
    redis: &nvbes_redis::RedisPool,
    key: &str,
    scope: &IdempotencyScope,
    request_hash: &str,
) -> Result<IdempotencyClaim, sqlx::Error> {
    let claim_key = idempotency_claim_key(key, scope);
    let release_token = idempotency_claim_value(request_hash);
    let (acquired, existing_hash) = nvbes_redis::idempotency::check_and_set(
        redis,
        &claim_key,
        &release_token,
        IN_FLIGHT_TTL_SECONDS,
    )
    .await
    .map_err(|e| sqlx::Error::Protocol(format!("Redis error: {e}")))?;

    if acquired {
        return Ok(IdempotencyClaim::Acquired { release_token });
    }

    Ok(IdempotencyClaim::InProgress {
        request_hash: existing_hash
            .as_deref()
            .map(idempotency_claim_request_hash)
            .unwrap_or_default()
            .to_string(),
    })
}

pub async fn release_idempotency_claim(
    redis: &nvbes_redis::RedisPool,
    key: &str,
    scope: &IdempotencyScope,
    release_token: &str,
) -> Result<(), sqlx::Error> {
    let claim_key = idempotency_claim_key(key, scope);
    nvbes_redis::idempotency::delete_if_value(redis, &claim_key, release_token)
        .await
        .map(|_| ())
        .map_err(|e| sqlx::Error::Protocol(format!("Redis error: {e}")))
}

fn idempotency_response_key(key: &str, scope: &IdempotencyScope) -> String {
    format!("nvbes:idem:{}:{}", scope.as_str(), key)
}

fn idempotency_claim_key(key: &str, scope: &IdempotencyScope) -> String {
    format!("nvbes:idem:claim:{}:{}", scope.as_str(), key)
}

fn idempotency_claim_value(request_hash: &str) -> String {
    format!("{request_hash}:{}", uuid::Uuid::new_v4())
}

fn idempotency_claim_request_hash(value: &str) -> &str {
    value.split_once(':').map_or(value, |(hash, _)| hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_redis_pool() -> Option<nvbes_redis::RedisPool> {
        let config = nvbes_redis::RedisConfig::from_env();
        let pool = nvbes_redis::connection::create_pool(&config).await.ok()?;
        if nvbes_redis::connection::health_check(&pool).await.is_err() {
            return None;
        }
        Some(pool)
    }

    #[tokio::test]
    async fn idempotency_round_trip_uses_redis() {
        let Some(redis) = test_redis_pool().await else {
            eprintln!("Skipping idempotency_round_trip_uses_redis: Redis is not reachable");
            return;
        };
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

    #[test]
    fn validate_key_trims_and_rejects_invalid_values() {
        assert_eq!(
            validate_key(" key-123 ").expect("key should be valid"),
            "key-123"
        );
        assert!(validate_key("   ").is_err());
        assert!(validate_key(&"a".repeat(IDEMPOTENCY_KEY_MAX_LEN + 1)).is_err());
    }

    #[test]
    fn request_signature_includes_method_path_and_body_hash() {
        let method = Method::POST;
        let uri = "/v1/workspaces/abc/objects?ignored=true"
            .parse::<Uri>()
            .expect("uri must parse");

        let signature = build_request_signature(&method, &uri, br#"{"name":"file"}"#);

        assert!(signature.starts_with("POST:/v1/workspaces/abc/objects:"));
        assert_eq!(
            signature.len(),
            "POST:/v1/workspaces/abc/objects:".len() + 64
        );
    }

    #[test]
    fn idempotency_keys_partition_claims_from_stored_responses() {
        let scope = IdempotencyScope("scope-a".to_string());

        assert_eq!(
            idempotency_response_key("key-a", &scope),
            "nvbes:idem:scope-a:key-a"
        );
        assert_eq!(
            idempotency_claim_key("key-a", &scope),
            "nvbes:idem:claim:scope-a:key-a"
        );
    }

    #[test]
    fn idempotency_claim_value_preserves_request_hash() {
        let request_hash = "request-hash-123";
        let value = idempotency_claim_value(request_hash);

        assert_eq!(idempotency_claim_request_hash(&value), request_hash);
    }

    #[test]
    fn derive_scope_changes_by_actor_and_route() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            "Bearer token-a".parse().expect("valid header"),
        );
        let method = Method::POST;
        let first_uri = "/v1/workspaces/a/objects"
            .parse::<Uri>()
            .expect("uri must parse");
        let second_uri = "/v1/workspaces/b/objects"
            .parse::<Uri>()
            .expect("uri must parse");

        let first = derive_scope(&headers, &method, &first_uri);
        let second = derive_scope(&headers, &method, &second_uri);

        assert_ne!(first.as_str(), second.as_str());
        assert_eq!(first.as_str().len(), 64);
    }
}
