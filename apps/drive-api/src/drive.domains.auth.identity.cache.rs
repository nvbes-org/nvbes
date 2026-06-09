use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use super::types::IdentityIntrospectionResponse;

static INTROSPECTION_CACHE: OnceLock<
    RwLock<HashMap<String, (IdentityIntrospectionResponse, Instant)>>,
> = OnceLock::new();

fn get_introspection_cache()
-> &'static RwLock<HashMap<String, (IdentityIntrospectionResponse, Instant)>> {
    INTROSPECTION_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn get_cached_introspection(token_hash: &str) -> Option<IdentityIntrospectionResponse> {
    let cache = get_introspection_cache().read().ok()?;
    if let Some((response, expires_at)) = cache.get(token_hash)
        && Instant::now() < *expires_at
    {
        return Some(response.clone());
    }
    None
}

pub fn set_cached_introspection(
    token_hash: String,
    response: IdentityIntrospectionResponse,
    ttl: Duration,
) {
    if let Ok(mut cache) = get_introspection_cache().write() {
        cache.insert(token_hash, (response, Instant::now() + ttl));
    }
}

pub fn invalidate_cached_session(session_id: &str) {
    if let Ok(mut cache) = get_introspection_cache().write() {
        cache.retain(|_, (resp, _)| resp.sid.as_deref() != Some(session_id));
    }
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
