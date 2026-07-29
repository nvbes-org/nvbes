use std::time::Duration;

use super::{
    EnqueueJobInput, claim_next_job,
    completion::{mark_job_failed_with_retention, mark_job_succeeded_with_retention},
    enqueue_job,
    keys::*,
    promote_due_jobs, recover_stale_jobs,
    retention::MIN_TERMINAL_RETENTION_SECONDS,
    test_support::with_isolated_queue,
};
use crate::RedisPool;
use redis::AsyncCommands;

const JOB_COUNT: usize = 205;
const TERMINAL_JOB_COUNT: usize = 128;

#[tokio::test]
async fn recovery_and_promotion_process_multiple_bounded_batches() {
    with_isolated_queue("volume", 8, |redis, queue| async move {
        for sequence in 0..JOB_COUNT {
            enqueue_job(
                &redis,
                EnqueueJobInput {
                    queue: queue.clone(),
                    job_type: "email.send".to_string(),
                    payload: serde_json::json!({ "sequence": sequence }),
                    idempotency_key: None,
                    max_attempts: 3,
                    overwrite_terminal: false,
                    job_id: None,
                },
            )
            .await
            .expect("enqueue");
        }
        for _ in 0..JOB_COUNT {
            claim_next_job(&redis, &[&queue], 1)
                .await
                .expect("claim succeeds")
                .expect("job is available");
        }

        let recovered = recover_stale_jobs(&redis, &queue, Duration::ZERO, Duration::ZERO)
            .await
            .expect("all recovery batches");
        assert_eq!(recovered.len(), JOB_COUNT);
        assert!(recovered.iter().all(|(_, outcome)| outcome == "failed"));

        let promoted = promote_due_jobs(&redis, &queue)
            .await
            .expect("all promotion batches");
        assert_eq!(promoted, JOB_COUNT as u64);

        let mut conn = redis.get().await.expect("redis connection");
        let pending: i64 = conn.llen(pending_key(&queue)).await.expect("pending depth");
        let running: i64 = conn
            .zcard(running_key(&queue))
            .await
            .expect("running depth");
        let delayed: i64 = conn
            .zcard(delayed_key(&queue))
            .await
            .expect("delayed depth");
        assert_eq!((pending, running, delayed), (JOB_COUNT as i64, 0, 0));
    })
    .await;
}

#[tokio::test]
async fn terminal_retention_remains_bounded_under_volume() {
    with_isolated_queue("retentionvolume", 8, |redis, queue| async move {
        for sequence in 0..TERMINAL_JOB_COUNT {
            enqueue_job(
                &redis,
                EnqueueJobInput {
                    queue: queue.clone(),
                    job_type: "email.send".to_string(),
                    payload: serde_json::json!({
                        "secret": format!("volume-secret-{sequence}"),
                    }),
                    idempotency_key: Some(format!("volume-{sequence}")),
                    max_attempts: 1,
                    overwrite_terminal: false,
                    job_id: None,
                },
            )
            .await
            .expect("enqueue");
        }

        for sequence in 0..TERMINAL_JOB_COUNT {
            let claimed = claim_next_job(&redis, &[&queue], 1)
                .await
                .expect("claim succeeds")
                .expect("job is available");
            if sequence % 2 == 0 {
                mark_job_succeeded_with_retention(&redis, &claimed, MIN_TERMINAL_RETENTION_SECONDS)
                    .await
                    .expect("complete");
            } else {
                mark_job_failed_with_retention(
                    &redis,
                    &claimed,
                    "volume-secret-must-not-survive",
                    false,
                    Duration::ZERO,
                    MIN_TERMINAL_RETENTION_SECONDS,
                )
                .await
                .expect("dead-letter");
            }
        }

        let mut conn = redis.get().await.expect("redis connection");
        let job_keys = scan_keys(&redis, &queue_key(&queue, "job:*")).await;
        let dedupe_keys = scan_keys(&redis, &queue_key(&queue, "dedupe:*")).await;
        assert_eq!(job_keys.len(), TERMINAL_JOB_COUNT);
        assert_eq!(dedupe_keys.len(), TERMINAL_JOB_COUNT);

        for key in job_keys.iter().chain(&dedupe_keys) {
            let ttl: i64 = conn.ttl(key).await.expect("terminal key TTL");
            assert!(
                (1..=MIN_TERMINAL_RETENTION_SECONDS as i64).contains(&ttl),
                "unexpected terminal TTL {ttl}"
            );
        }
        let raw_jobs: Vec<String> = redis::cmd("MGET")
            .arg(&job_keys)
            .query_async(&mut *conn)
            .await
            .expect("read terminal tombstones");
        assert!(raw_jobs.iter().all(|raw| !raw.contains("volume-secret")));

        let dead_letter_depth: i64 = conn
            .zcard(dead_letter_key(&queue))
            .await
            .expect("dead-letter depth");
        assert_eq!(dead_letter_depth, (TERMINAL_JOB_COUNT / 2) as i64);
        let dead_letter_ttl: i64 = conn
            .ttl(dead_letter_key(&queue))
            .await
            .expect("dead-letter TTL");
        assert!((1..=MIN_TERMINAL_RETENTION_SECONDS as i64).contains(&dead_letter_ttl));
    })
    .await;
}

async fn scan_keys(pool: &RedisPool, pattern: &str) -> Vec<String> {
    let mut conn = pool.get().await.expect("redis connection");
    let mut cursor = 0_u64;
    let mut keys = Vec::new();
    loop {
        let (next_cursor, page): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(250)
            .query_async(&mut *conn)
            .await
            .expect("scan terminal keys");
        keys.extend(page);
        if next_cursor == 0 {
            return keys;
        }
        cursor = next_cursor;
    }
}
