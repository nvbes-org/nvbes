use super::*;

async fn limiter() -> RateLimiter {
    let db = nvbes_test_utils::postgres::test_pool().await;
    sqlx::raw_sql(SCHEMA).execute(&db).await.unwrap();
    RateLimiter::new(db)
}

#[tokio::test]
async fn concurrent_requests_share_a_single_limit() {
    let limiter = limiter().await;
    let window = Duration::from_secs(60);
    let (a, b) = tokio::join!(
        limiter.check("login", "user", 1, window),
        limiter.check("login", "user", 1, window)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let error = a.err().or_else(|| b.err()).unwrap();
    assert_eq!(error.code, "rate_limited");
    let independent = limiter.check("login", "other", 2, window).await.unwrap();
    assert_eq!(independent.remaining, 1);
    assert!((1..=60).contains(&independent.reset));
    assert!(limiter.check("export", "user", 1, window).await.is_ok());
}

#[tokio::test]
async fn expiry_resets_counter_and_cleanup_preserves_live_windows() {
    let limiter = limiter().await;
    let window = Duration::from_secs(60);
    limiter.check("login", "user", 1, window).await.unwrap();
    sqlx::query(
        "UPDATE rate_limit_windows SET expires_at = clock_timestamp() - interval '1 second'",
    )
    .execute(&limiter.db)
    .await
    .unwrap();
    assert!(limiter.check("login", "user", 1, window).await.is_ok());
    assert_eq!(limiter.cleanup_expired().await.unwrap(), 0);
    sqlx::query(
        "UPDATE rate_limit_windows SET expires_at = clock_timestamp() - interval '1 second'",
    )
    .execute(&limiter.db)
    .await
    .unwrap();
    assert_eq!(limiter.cleanup_expired().await.unwrap(), 1);
}

#[tokio::test]
async fn invalid_policy_and_database_failure_never_allow_requests() {
    let limiter = limiter().await;
    for (hits, window) in [(0, 60), (1, 0), (1, 86401), (usize::MAX, 60)] {
        assert_eq!(
            limiter
                .check("login", "user", hits, Duration::from_secs(window))
                .await
                .unwrap_err()
                .code,
            "rate_limiter_config"
        );
    }
    limiter.db.close().await;
    assert_eq!(
        limiter
            .check("login", "user", 1, Duration::from_secs(60))
            .await
            .unwrap_err()
            .code,
        "rate_limiter_error"
    );
}
