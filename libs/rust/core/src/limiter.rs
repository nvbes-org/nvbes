use crate::http::client_ip::client_ip;
use crate::http::error::AppError;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

static RATELIMIT_LIMIT: HeaderName = HeaderName::from_static("ratelimit-limit");
static RATELIMIT_REMAINING: HeaderName = HeaderName::from_static("ratelimit-remaining");
static RATELIMIT_RESET: HeaderName = HeaderName::from_static("ratelimit-reset");

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: usize,
    pub remaining: usize,
    pub reset: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitRule<'a> {
    pub key: &'a str,
    pub max_hits: usize,
    pub window: Duration,
}

impl RateLimitInfo {
    pub fn append_headers(&self, headers: &mut HeaderMap) {
        if let Ok(v) = HeaderValue::from_str(&self.limit.to_string()) {
            headers.insert(&RATELIMIT_LIMIT, v);
        }
        if let Ok(v) = HeaderValue::from_str(&self.remaining.to_string()) {
            headers.insert(&RATELIMIT_REMAINING, v);
        }
        if let Ok(v) = HeaderValue::from_str(&self.reset.to_string()) {
            headers.insert(&RATELIMIT_RESET, v);
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    redis: nvbes_redis::RedisPool,
}

impl RateLimiter {
    pub fn new(redis: nvbes_redis::RedisPool) -> Self {
        Self { redis }
    }

    pub async fn check(
        &self,
        action: &str,
        key: &str,
        max_hits: usize,
        window: Duration,
    ) -> Result<RateLimitInfo, AppError> {
        let max_hits_u64 = max_hits as u64;
        let window_secs = window.as_secs();
        let result = nvbes_redis::rate_limit::check_rate_limit(
            &self.redis,
            "default",
            action,
            key,
            max_hits_u64,
            window_secs,
        )
        .await
        .map_err(|error| {
            tracing::error!(%error, action, key, "Redis rate limiter failed");
            AppError::internal("rate_limiter_error", "Rate limiter is unavailable.")
        })?;

        let remaining = max_hits.saturating_sub(result.current as usize);
        let info = RateLimitInfo {
            limit: max_hits,
            remaining,
            reset: result.ttl_seconds,
        };
        if !result.allowed {
            return Err(AppError::too_many_requests(
                "rate_limited",
                "Too many requests. Try again later.",
                Some(result.ttl_seconds.max(1)),
                Some(info),
            ));
        }
        Ok(info)
    }
}

pub async fn check_rate_limit(
    redis: &nvbes_redis::RedisPool,
    action: &str,
    key: &str,
    max_hits: usize,
    window: Duration,
) -> Result<RateLimitInfo, AppError> {
    RateLimiter::new(redis.clone())
        .check(action, key, max_hits, window)
        .await
}

pub async fn check_dual_rate_limit(
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    action: &str,
    scoped_key: &str,
    ip_hits: usize,
    scoped_hits: usize,
    window: Duration,
) -> Result<(), AppError> {
    let ip_key = client_ip(headers).unwrap_or_else(|| "unknown".to_string());
    check_rate_limit(redis, action, &format!("ip:{ip_key}"), ip_hits, window).await?;
    check_rate_limit(
        redis,
        action,
        &format!("key:{scoped_key}"),
        scoped_hits,
        window,
    )
    .await?;
    Ok(())
}

pub async fn check_rate_limit_pair(
    redis: &nvbes_redis::RedisPool,
    action: &str,
    first: RateLimitRule<'_>,
    second: RateLimitRule<'_>,
) -> Result<(), AppError> {
    check_rate_limit(redis, action, first.key, first.max_hits, first.window).await?;
    check_rate_limit(redis, action, second.key, second.max_hits, second.window).await?;
    Ok(())
}

pub async fn check_rate_limit_rules(
    redis: &nvbes_redis::RedisPool,
    action: &str,
    rules: &[RateLimitRule<'_>],
) -> Result<(), AppError> {
    for rule in rules {
        check_rate_limit(redis, action, rule.key, rule.max_hits, rule.window).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn test_redis_pool() -> nvbes_redis::RedisPool {
        let config = nvbes_redis::RedisConfig::from_env();
        nvbes_redis::connection::create_pool(&config)
            .await
            .expect("redis pool")
    }

    #[tokio::test]
    async fn rate_limiter_blocks_after_limit() {
        let redis = test_redis_pool().await;
        let limiter = RateLimiter::new(redis);
        let action = format!("core_rate_limit_test_{}", uuid::Uuid::new_v4());
        let key = "key:rate-limit";

        limiter
            .check(&action, key, 1, Duration::from_secs(60))
            .await
            .expect("first request should pass");

        let error = limiter
            .check(&action, key, 1, Duration::from_secs(60))
            .await
            .expect_err("second request should be rate limited");

        assert_eq!(error.code, "rate_limited");
    }
}
