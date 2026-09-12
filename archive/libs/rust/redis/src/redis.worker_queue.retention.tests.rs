use std::time::Duration;

use redis::AsyncCommands;

use super::{
    EnqueueJobInput, QueuedJob, claim_next_job,
    completion::{mark_job_failed_with_retention, mark_job_succeeded_with_retention},
    enqueue_job, get_job,
    keys::*,
    recover_stale_jobs,
    retention::{MIN_TERMINAL_RETENTION_SECONDS, REDACTED_TERMINAL_ERROR},
    test_support::with_isolated_queue,
};
use crate::RedisPool;

const SECRET_PAYLOAD: &str = "sensitive-email-token";
const IDEMPOTENCY_KEY: &str = "retention-contract";

fn input(queue: &str, max_attempts: u32) -> EnqueueJobInput {
    EnqueueJobInput {
        queue: queue.to_string(),
        job_type: "email.send".to_string(),
        payload: serde_json::json!({
            "to_email": "private@example.test",
            "token": SECRET_PAYLOAD,
        }),
        idempotency_key: Some(IDEMPOTENCY_KEY.to_string()),
        max_attempts,
        overwrite_terminal: false,
        job_id: None,
    }
}

async fn claim(pool: &RedisPool, queue: &str) -> QueuedJob {
    claim_next_job(pool, &[queue], 1)
        .await
        .expect("claim succeeds")
        .expect("job exists")
}

async fn ttl(pool: &RedisPool, key: &str) -> i64 {
    let mut connection = pool.get().await.expect("redis connection");
    connection.ttl(key).await.expect("read TTL")
}

async fn raw_job(pool: &RedisPool, queue: &str, job: &QueuedJob) -> String {
    let mut connection = pool.get().await.expect("redis connection");
    connection
        .get(job_key(queue, job.id))
        .await
        .expect("read raw job")
}

#[tokio::test]
async fn successful_completion_scrubs_secrets_and_retains_only_bounded_tombstones() {
    with_isolated_queue("successretention", 4, |redis, queue| async move {
        let job_id = enqueue_job(&redis, input(&queue, 3))
            .await
            .expect("enqueue");
        let claimed = claim(&redis, &queue).await;

        mark_job_succeeded_with_retention(&redis, &claimed, MIN_TERMINAL_RETENTION_SECONDS)
            .await
            .expect("complete");

        let stored = get_job(&redis, &queue, job_id)
            .await
            .expect("lookup")
            .expect("terminal tombstone");
        assert_eq!(stored.status, STATUS_SUCCEEDED);
        assert_eq!(stored.payload, serde_json::json!({}));
        assert_eq!(stored.idempotency_key, None);
        assert_eq!(stored.result, Some(serde_json::json!({ "redacted": true })));
        let raw = raw_job(&redis, &queue, &stored).await;
        assert!(!raw.contains(SECRET_PAYLOAD));
        assert!(!raw.contains("private@example.test"));

        let job_ttl = ttl(&redis, &job_key(&queue, job_id)).await;
        let dedupe_ttl = ttl(&redis, &dedupe_key(&queue, "email.send", IDEMPOTENCY_KEY)).await;
        assert!((1..=MIN_TERMINAL_RETENTION_SECONDS as i64).contains(&job_ttl));
        assert!((1..=MIN_TERMINAL_RETENTION_SECONDS as i64).contains(&dedupe_ttl));
    })
    .await;
}

#[tokio::test]
async fn dead_letter_scrubs_payload_error_and_expires_all_terminal_indexes() {
    const RETENTION_SECONDS: u64 = 2;
    with_isolated_queue("deadretention", 4, |redis, queue| async move {
        let job_id = enqueue_job(&redis, input(&queue, 1))
            .await
            .expect("enqueue");
        let claimed = claim(&redis, &queue).await;

        mark_job_failed_with_retention(
            &redis,
            &claimed,
            "provider rejected sensitive-email-token",
            false,
            Duration::ZERO,
            RETENTION_SECONDS,
        )
        .await
        .expect("dead-letter");

        let stored = get_job(&redis, &queue, job_id)
            .await
            .expect("lookup")
            .expect("terminal tombstone");
        assert_eq!(stored.status, STATUS_DEAD_LETTER);
        assert_eq!(stored.payload, serde_json::json!({}));
        assert_eq!(stored.idempotency_key, None);
        assert_eq!(stored.last_error.as_deref(), Some(REDACTED_TERMINAL_ERROR));
        assert!(
            !raw_job(&redis, &queue, &stored)
                .await
                .contains(SECRET_PAYLOAD)
        );

        let dedupe = dedupe_key(&queue, "email.send", IDEMPOTENCY_KEY);
        assert!(
            (1..=RETENTION_SECONDS as i64).contains(&ttl(&redis, &job_key(&queue, job_id)).await)
        );
        assert!((1..=RETENTION_SECONDS as i64).contains(&ttl(&redis, &dedupe).await));
        assert!(
            (1..=RETENTION_SECONDS as i64).contains(&ttl(&redis, &dead_letter_key(&queue)).await)
        );

        tokio::time::sleep(Duration::from_millis(2_200)).await;
        assert!(
            get_job(&redis, &queue, job_id)
                .await
                .expect("lookup after expiry")
                .is_none()
        );
        let mut connection = redis.get().await.expect("redis connection");
        let dedupe_exists: bool = connection.exists(dedupe).await.expect("dedupe expiry");
        let dead_letter_exists: bool = connection
            .exists(dead_letter_key(&queue))
            .await
            .expect("dead-letter index expiry");
        assert!(!dedupe_exists);
        assert!(!dead_letter_exists);
    })
    .await;
}

#[tokio::test]
async fn retryable_failure_keeps_payload_without_applying_terminal_retention() {
    with_isolated_queue("retryretention", 4, |redis, queue| async move {
        let job_id = enqueue_job(&redis, input(&queue, 3))
            .await
            .expect("enqueue");
        let claimed = claim(&redis, &queue).await;
        mark_job_failed_with_retention(
            &redis,
            &claimed,
            "temporary provider error",
            true,
            Duration::from_secs(30),
            MIN_TERMINAL_RETENTION_SECONDS,
        )
        .await
        .expect("schedule retry");

        let stored = get_job(&redis, &queue, job_id)
            .await
            .expect("lookup")
            .expect("retryable job");
        assert_eq!(stored.status, STATUS_FAILED);
        assert_eq!(stored.payload["token"], SECRET_PAYLOAD);
        assert_eq!(ttl(&redis, &job_key(&queue, job_id)).await, -1);
        assert_eq!(
            ttl(&redis, &dedupe_key(&queue, "email.send", IDEMPOTENCY_KEY),).await,
            -1
        );
    })
    .await;
}

#[tokio::test]
async fn stale_recovery_dead_letter_uses_the_same_scrub_and_retention_contract() {
    with_isolated_queue("recoveryretention", 4, |redis, queue| async move {
        let job_id = enqueue_job(&redis, input(&queue, 1))
            .await
            .expect("enqueue");
        claim(&redis, &queue).await;

        let recovered = recover_stale_jobs(&redis, &queue, Duration::ZERO, Duration::ZERO)
            .await
            .expect("recover");
        assert_eq!(
            recovered,
            vec![("email.send".to_string(), STATUS_DEAD_LETTER.to_string())]
        );
        let stored = get_job(&redis, &queue, job_id)
            .await
            .expect("lookup")
            .expect("recovered tombstone");
        assert_eq!(stored.payload, serde_json::json!({}));
        assert_eq!(stored.last_error.as_deref(), Some(REDACTED_TERMINAL_ERROR));
        assert!(ttl(&redis, &job_key(&queue, job_id)).await > 0);
        assert!(ttl(&redis, &dedupe_key(&queue, "email.send", IDEMPOTENCY_KEY),).await > 0);
    })
    .await;
}

#[tokio::test]
async fn first_terminal_transition_replaces_the_legacy_unbounded_set_index() {
    with_isolated_queue("legacyindex", 4, |redis, queue| async move {
        let job_id = enqueue_job(&redis, input(&queue, 1))
            .await
            .expect("enqueue");
        let claimed = claim(&redis, &queue).await;
        let mut connection = redis.get().await.expect("redis connection");
        let _: usize = connection
            .sadd(dead_letter_key(&queue), "legacy-job-id")
            .await
            .expect("seed legacy set");
        drop(connection);

        mark_job_failed_with_retention(
            &redis,
            &claimed,
            "terminal",
            false,
            Duration::ZERO,
            MIN_TERMINAL_RETENTION_SECONDS,
        )
        .await
        .expect("migrate and dead-letter");

        let mut connection = redis.get().await.expect("redis connection");
        let key_type: String = redis::cmd("TYPE")
            .arg(dead_letter_key(&queue))
            .query_async(&mut *connection)
            .await
            .expect("read index type");
        let members: Vec<String> = connection
            .zrange(dead_letter_key(&queue), 0, -1)
            .await
            .expect("read migrated index");
        assert_eq!(key_type, "zset");
        assert_eq!(members, vec![job_id.to_string()]);
    })
    .await;
}
