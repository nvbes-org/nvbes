use base64::Engine;
use nvbes_redis::{RedisError, RedisPool};
use rand_core::RngCore;

#[derive(Debug)]
pub struct DpopNonceStore {
    redis: RedisPool,
    ttl_seconds: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DpopNonceStoreError {
    #[error("Redis nonce store error: {0}")]
    Redis(#[from] RedisError),
}

impl DpopNonceStore {
    pub fn new(redis: RedisPool, ttl_seconds: i64) -> Self {
        Self {
            redis,
            ttl_seconds: ttl_seconds.max(1) as u64,
        }
    }

    pub async fn generate(&self) -> Result<String, DpopNonceStoreError> {
        let nonce = generate_random_nonce();
        let redis_key = redis_nonce_key(&nonce);
        nvbes_redis::cache::cache_set(&self.redis, &redis_key, "1", self.ttl_seconds).await?;
        Ok(nonce)
    }

    pub async fn is_valid(&self, nonce: &str) -> Result<bool, DpopNonceStoreError> {
        let redis_key = redis_nonce_key(nonce);
        Ok(nvbes_redis::cache::cache_get(&self.redis, &redis_key)
            .await?
            .is_some())
    }

    pub async fn consume(&self, nonce: &str) -> Result<bool, DpopNonceStoreError> {
        let redis_key = redis_nonce_key(nonce);
        Ok(nvbes_redis::cache::cache_take(&self.redis, &redis_key)
            .await?
            .is_some())
    }

    pub async fn register_jti(
        &self,
        jti: &str,
        jkt: &str,
        ttl_seconds: u64,
    ) -> Result<bool, DpopNonceStoreError> {
        let redis_key = format!("nvbes:dpop:jti:{jkt}:{jti}");
        Ok(nvbes_redis::cache::cache_set_nx(&self.redis, &redis_key, "1", ttl_seconds).await?)
    }
}

fn redis_nonce_key(nonce: &str) -> String {
    format!("nvbes:dpop:nonce:{nonce}")
}

fn generate_random_nonce() -> String {
    let mut bytes = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_redis_pool() -> Option<RedisPool> {
        let redis_config = nvbes_redis::RedisConfig::from_env();
        let pool = nvbes_redis::connection::create_pool(&redis_config)
            .await
            .ok()?;
        if nvbes_redis::connection::health_check(&pool).await.is_err() {
            return None;
        }
        Some(pool)
    }

    #[tokio::test]
    async fn nonce_validation_and_consumption() {
        let Some(pool) = test_redis_pool().await else {
            eprintln!("Skipping nonce_validation_and_consumption: Redis is not reachable");
            return;
        };
        let store = DpopNonceStore::new(pool, 300);
        let nonce = store.generate().await.expect("nonce");
        assert!(store.is_valid(&nonce).await.expect("valid"));
        assert!(store.consume(&nonce).await.expect("consume"));
        assert!(!store.is_valid(&nonce).await.expect("valid"));
    }

    #[tokio::test]
    async fn nonces_are_unique() {
        let Some(pool) = test_redis_pool().await else {
            eprintln!("Skipping nonces_are_unique: Redis is not reachable");
            return;
        };
        let store = DpopNonceStore::new(pool, 300);
        let n1 = store.generate().await.expect("nonce");
        let n2 = store.generate().await.expect("nonce");
        assert_ne!(n1, n2);
    }

    #[tokio::test]
    async fn a_dpop_jti_cannot_be_replayed_with_the_same_key() {
        let Some(pool) = test_redis_pool().await else {
            eprintln!(
                "Skipping a_dpop_jti_cannot_be_replayed_with_the_same_key: Redis is not reachable"
            );
            return;
        };
        let store = DpopNonceStore::new(pool, 300);
        let jti = uuid::Uuid::new_v4().to_string();
        let jkt = uuid::Uuid::new_v4().to_string();

        assert!(
            store
                .register_jti(&jti, &jkt, 300)
                .await
                .expect("first proof")
        );
        assert!(
            !store
                .register_jti(&jti, &jkt, 300)
                .await
                .expect("replayed proof")
        );
    }
}
