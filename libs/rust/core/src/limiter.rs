use crate::http::client_ip::client_ip;
use crate::http::error::AppError;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Apply with a migration role before constructing a limiter with a DML-only pool.
pub const SCHEMA: &str = include_str!("limiter.postgres.sql");

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
    db: sqlx::PgPool,
}

impl RateLimiter {
    pub fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }

    pub async fn check(
        &self,
        action: &str,
        key: &str,
        max_hits: usize,
        window: Duration,
    ) -> Result<RateLimitInfo, AppError> {
        let window_secs = window.as_secs();
        if !(1..=1_000_000_000).contains(&max_hits)
            || !(1..=86400).contains(&window_secs)
            || action.is_empty()
            || action.len() > 255
        {
            return Err(AppError::internal(
                "rate_limiter_config",
                "Invalid rate limit policy.",
            ));
        }
        let (current, ttl_seconds): (i64, i64) = sqlx::query_as(
            "INSERT INTO rate_limit_windows (action, key_hash, hits, expires_at)
             VALUES ($1, $2, 1, clock_timestamp() + $3 * interval '1 second')
             ON CONFLICT (action, key_hash) DO UPDATE SET (hits, expires_at) = (
               WITH instant AS MATERIALIZED (SELECT clock_timestamp() AS observed_at)
               SELECT CASE WHEN rate_limit_windows.expires_at <= observed_at THEN 1
                      ELSE LEAST(rate_limit_windows.hits + 1, 1000000001) END,
                 CASE WHEN rate_limit_windows.expires_at <= observed_at
                 THEN observed_at + $3 * interval '1 second'
                 ELSE rate_limit_windows.expires_at END FROM instant)
             RETURNING hits, GREATEST(1, CEIL(EXTRACT(EPOCH FROM (expires_at - clock_timestamp()))))::bigint")
        .bind(action).bind(Sha256::digest(key.as_bytes()).to_vec()).bind(window_secs as i64)
        .fetch_one(&self.db).await.map_err(|error| {
            tracing::error!(%error, "PostgreSQL rate limiter failed");
            AppError::internal("rate_limiter_error", "Rate limiter is unavailable.")
        })?;

        let remaining = max_hits.saturating_sub(current as usize);
        let info = RateLimitInfo {
            limit: max_hits,
            remaining,
            reset: ttl_seconds as u64,
        };
        if current > max_hits as i64 {
            return Err(AppError::too_many_requests(
                "rate_limited",
                "Too many requests. Try again later.",
                Some(ttl_seconds as u64),
                Some(info),
            ));
        }
        Ok(info)
    }

    /// Bound retention work; the caller schedules this on its maintenance loop.
    pub async fn cleanup_expired(&self) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query(
            "DELETE FROM rate_limit_windows WHERE (action, key_hash) IN
            (SELECT action, key_hash FROM rate_limit_windows WHERE expires_at <= clock_timestamp()
             ORDER BY expires_at LIMIT 1000 FOR UPDATE SKIP LOCKED)",
        )
        .execute(&self.db)
        .await?
        .rows_affected())
    }
}

pub async fn check_rate_limit(
    db: &sqlx::PgPool,
    action: &str,
    key: &str,
    max_hits: usize,
    window: Duration,
) -> Result<RateLimitInfo, AppError> {
    RateLimiter::new(db.clone())
        .check(action, key, max_hits, window)
        .await
}

pub async fn check_dual_rate_limit(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    action: &str,
    scoped_key: &str,
    ip_hits: usize,
    scoped_hits: usize,
    window: Duration,
) -> Result<(), AppError> {
    let ip_key = client_ip(headers).unwrap_or_else(|| "unknown".to_string());
    check_rate_limit(db, action, &format!("ip:{ip_key}"), ip_hits, window).await?;
    check_rate_limit(
        db,
        action,
        &format!("key:{scoped_key}"),
        scoped_hits,
        window,
    )
    .await?;
    Ok(())
}

pub async fn check_rate_limit_pair(
    db: &sqlx::PgPool,
    action: &str,
    first: RateLimitRule<'_>,
    second: RateLimitRule<'_>,
) -> Result<(), AppError> {
    check_rate_limit(db, action, first.key, first.max_hits, first.window).await?;
    check_rate_limit(db, action, second.key, second.max_hits, second.window).await?;
    Ok(())
}

pub async fn check_rate_limit_rules(
    db: &sqlx::PgPool,
    action: &str,
    rules: &[RateLimitRule<'_>],
) -> Result<(), AppError> {
    for rule in rules {
        check_rate_limit(db, action, rule.key, rule.max_hits, rule.window).await?;
    }
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::time::Duration;

    #[test]
    fn rate_limit_info_appends_standard_headers() {
        let info = RateLimitInfo {
            limit: 100,
            remaining: 99,
            reset: 60,
        };
        let mut headers = HeaderMap::new();
        info.append_headers(&mut headers);

        assert_eq!(headers.get("ratelimit-limit").unwrap(), "100");
        assert_eq!(headers.get("ratelimit-remaining").unwrap(), "99");
        assert_eq!(headers.get("ratelimit-reset").unwrap(), "60");
    }

    #[tokio::test]
    async fn check_rejects_invalid_policy_before_touching_storage() {
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/unused")
            .expect("lazy pool");
        let limiter = RateLimiter::new(db.clone());

        let cases = [
            (0usize, Duration::from_secs(60), "action"),
            (1usize, Duration::from_secs(0), "action"),
            (1usize, Duration::from_secs(60), ""),
            (1usize, Duration::from_secs(60), &"a".repeat(256)),
        ];
        for (max_hits, window, action) in cases {
            let err = limiter
                .check(action, "key", max_hits, window)
                .await
                .expect_err("invalid policy");
            assert_eq!(err.code, "rate_limiter_config");
        }

        check_rate_limit_rules(&db, "noop", &[])
            .await
            .expect("empty rules");
    }
}

#[cfg(test)]
#[path = "limiter.postgres.tests.rs"]
mod postgres_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn rate_limiter_blocks_after_limit() {
        let db = nvbes_test_utils::postgres::test_pool().await;
        sqlx::raw_sql(SCHEMA).execute(&db).await.unwrap();
        let limiter = RateLimiter::new(db);
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
