use std::{collections::HashSet, sync::Arc, time::Duration};

use redis::AsyncCommands;
use tokio::sync::Barrier;

use super::{
    EnqueueJobInput, QueuedJob, claim_next_job, enqueue_job, get_job, keys::*, mark_job_failed,
    mark_job_succeeded, promote_due_jobs, recover_stale_jobs, test_support::with_isolated_queue,
};
use crate::{RedisError, RedisPool};

fn input(queue: &str, sequence: usize, idempotency_key: Option<&str>) -> EnqueueJobInput {
    EnqueueJobInput {
        queue: queue.to_string(),
        job_type: "email.send".to_string(),
        payload: serde_json::json!({ "sequence": sequence }),
        idempotency_key: idempotency_key.map(str::to_string),
        max_attempts: 3,
        overwrite_terminal: false,
        job_id: None,
    }
}

async fn index_depths(pool: &RedisPool, queue: &str) -> (i64, i64, i64, i64) {
    let mut conn = pool.get().await.expect("redis connection");
    let pending = conn.llen(pending_key(queue)).await.expect("pending depth");
    let running = conn.zcard(running_key(queue)).await.expect("running depth");
    let delayed = conn.zcard(delayed_key(queue)).await.expect("delayed depth");
    let dead = conn
        .zcard(dead_letter_key(queue))
        .await
        .expect("dead-letter depth");
    (pending, running, delayed, dead)
}

async fn claim(pool: &RedisPool, queue: &str) -> QueuedJob {
    claim_next_job(pool, &[queue], 1)
        .await
        .expect("claim succeeds")
        .expect("job is available")
}

#[tokio::test]
async fn concurrent_idempotent_enqueue_creates_one_job_and_one_pointer() {
    const PRODUCERS: usize = 32;
    with_isolated_queue("dedupe", 16, |redis, queue| async move {
        let barrier = Arc::new(Barrier::new(PRODUCERS));
        let mut tasks = Vec::with_capacity(PRODUCERS);

        for sequence in 0..PRODUCERS {
            let pool = redis.clone();
            let queue = queue.clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                enqueue_job(&pool, input(&queue, sequence, Some("same-command")))
                    .await
                    .expect("enqueue")
            }));
        }

        let mut ids = HashSet::new();
        for task in tasks {
            ids.insert(task.await.expect("producer task"));
        }
        assert_eq!(ids.len(), 1, "all producers must observe the same job");
        assert_eq!(index_depths(&redis, &queue).await, (1, 0, 0, 0));

        let claimed = claim(&redis, &queue).await;
        assert!(ids.contains(&claimed.id));
        assert_eq!(index_depths(&redis, &queue).await, (0, 1, 0, 0));
    })
    .await;
}

#[tokio::test]
async fn concurrent_consumers_claim_each_job_exactly_once() {
    const JOBS: usize = 24;
    with_isolated_queue("claim", 16, |redis, queue| async move {
        let mut expected = HashSet::new();
        for sequence in 0..JOBS {
            expected.insert(
                enqueue_job(&redis, input(&queue, sequence, None))
                    .await
                    .expect("enqueue"),
            );
        }

        let mut consumers = Vec::with_capacity(JOBS);
        for _ in 0..JOBS {
            let pool = redis.clone();
            let queue = queue.clone();
            consumers.push(tokio::spawn(async move { claim(&pool, &queue).await }));
        }
        let mut claimed = HashSet::new();
        for consumer in consumers {
            claimed.insert(consumer.await.expect("consumer task").id);
        }

        assert_eq!(claimed, expected);
        assert_eq!(index_depths(&redis, &queue).await, (0, JOBS as i64, 0, 0));
    })
    .await;
}

#[tokio::test]
async fn retry_uses_a_new_lease_and_rejects_stale_acknowledgement() {
    with_isolated_queue("retry", 16, |redis, queue| async move {
        let id = enqueue_job(&redis, input(&queue, 1, None))
            .await
            .expect("enqueue");
        let first_claim = claim(&redis, &queue).await;

        mark_job_failed(
            &redis,
            &first_claim,
            "temporary failure",
            true,
            Duration::ZERO,
        )
        .await
        .expect("retry scheduled");
        assert_eq!(promote_due_jobs(&redis, &queue).await.expect("promote"), 1);
        let second_claim = claim(&redis, &queue).await;
        assert_eq!(second_claim.id, id);
        assert_eq!(second_claim.attempts, 2);

        let stale_ack = mark_job_succeeded(&redis, &first_claim, serde_json::json!({})).await;
        assert!(matches!(stale_ack, Err(RedisError::JobLeaseLost { .. })));
        mark_job_succeeded(&redis, &second_claim, serde_json::json!({ "sent": true }))
            .await
            .expect("current lease succeeds");

        let stored = get_job(&redis, &queue, id)
            .await
            .expect("job lookup")
            .expect("stored job");
        assert_eq!(stored.status, STATUS_SUCCEEDED);
        assert_eq!(stored.attempts, 2);
        assert_eq!(index_depths(&redis, &queue).await, (0, 0, 0, 0));
    })
    .await;
}

#[tokio::test]
async fn crashed_claim_is_recovered_without_losing_the_job() {
    with_isolated_queue("recovery", 16, |redis, queue| async move {
        let mut job_input = input(&queue, 1, None);
        job_input.payload = serde_json::json!({
            "empty_object": {},
            "empty_array": [],
            "nested": { "items": [] },
        });
        let expected_payload = job_input.payload.clone();
        let id = enqueue_job(&redis, job_input).await.expect("enqueue");
        let abandoned_claim = claim(&redis, &queue).await;

        let recovered = recover_stale_jobs(&redis, &queue, Duration::ZERO, Duration::ZERO)
            .await
            .expect("recovery");
        assert_eq!(
            recovered,
            vec![("email.send".to_string(), "failed".to_string())]
        );
        let reclaimed = claim(&redis, &queue).await;
        assert_eq!(reclaimed.id, id);
        assert_eq!(reclaimed.attempts, 2);
        assert_eq!(reclaimed.payload, expected_payload);

        let stale_ack = mark_job_succeeded(&redis, &abandoned_claim, serde_json::json!({})).await;
        assert!(matches!(stale_ack, Err(RedisError::JobLeaseLost { .. })));
        mark_job_succeeded(&redis, &reclaimed, serde_json::json!({}))
            .await
            .expect("reclaimed lease succeeds");
    })
    .await;
}

#[tokio::test]
async fn exhausted_retry_budget_moves_job_to_dead_letter_once() {
    with_isolated_queue("deadletter", 16, |redis, queue| async move {
        let mut job_input = input(&queue, 1, Some("terminal"));
        job_input.max_attempts = 1;
        let id = enqueue_job(&redis, job_input).await.expect("enqueue");
        let claimed = claim(&redis, &queue).await;

        mark_job_failed(&redis, &claimed, "permanent failure", true, Duration::ZERO)
            .await
            .expect("dead-letter");
        assert_eq!(index_depths(&redis, &queue).await, (0, 0, 0, 1));
        assert_eq!(
            get_job(&redis, &queue, id)
                .await
                .expect("job lookup")
                .expect("stored job")
                .status,
            STATUS_DEAD_LETTER
        );
        assert!(
            recover_stale_jobs(&redis, &queue, Duration::ZERO, Duration::ZERO)
                .await
                .expect("recovery")
                .is_empty()
        );
    })
    .await;
}

#[tokio::test]
async fn terminal_deduplicated_job_can_be_requeued_atomically() {
    with_isolated_queue("overwrite", 16, |redis, queue| async move {
        let first_id = enqueue_job(&redis, input(&queue, 1, Some("replayable")))
            .await
            .expect("enqueue");
        let claimed = claim(&redis, &queue).await;
        mark_job_failed(&redis, &claimed, "invalid", false, Duration::ZERO)
            .await
            .expect("dead-letter");
        let dedupe = dedupe_key(&queue, "email.send", "replayable");
        let mut conn = redis.get().await.expect("redis connection");
        let terminal_job_ttl: i64 = conn
            .ttl(job_key(&queue, first_id))
            .await
            .expect("terminal job TTL");
        let terminal_dedupe_ttl: i64 = conn.ttl(&dedupe).await.expect("terminal dedupe TTL");
        assert!(terminal_job_ttl > 0);
        assert!(terminal_dedupe_ttl > 0);
        drop(conn);

        let mut replacement = input(&queue, 2, Some("replayable"));
        replacement.overwrite_terminal = true;
        let replacement_id = enqueue_job(&redis, replacement)
            .await
            .expect("replacement enqueue");
        assert_eq!(replacement_id, first_id);
        assert_eq!(index_depths(&redis, &queue).await, (1, 0, 0, 0));
        let mut conn = redis.get().await.expect("redis connection");
        let revived_job_ttl: i64 = conn
            .ttl(job_key(&queue, first_id))
            .await
            .expect("revived job TTL");
        let revived_dedupe_ttl: i64 = conn.ttl(dedupe).await.expect("revived dedupe TTL");
        assert_eq!(revived_job_ttl, -1);
        assert_eq!(revived_dedupe_ttl, -1);

        let replayed = claim(&redis, &queue).await;
        assert_eq!(replayed.attempts, 1);
        assert_eq!(replayed.payload["sequence"], 2);
    })
    .await;
}

#[tokio::test]
async fn completion_and_recovery_race_preserves_a_single_authoritative_state() {
    const ROUNDS: usize = 20;
    for round in 0..ROUNDS {
        with_isolated_queue("race", 16, move |redis, queue| async move {
            let id = enqueue_job(&redis, input(&queue, round, None))
                .await
                .expect("enqueue");
            let claimed = claim(&redis, &queue).await;

            let (completed, recovered) = tokio::join!(
                mark_job_succeeded(&redis, &claimed, serde_json::json!({})),
                recover_stale_jobs(&redis, &queue, Duration::ZERO, Duration::ZERO)
            );
            let stored = get_job(&redis, &queue, id)
                .await
                .expect("job lookup")
                .expect("stored job");
            let depths = index_depths(&redis, &queue).await;

            match stored.status.as_str() {
                STATUS_SUCCEEDED => {
                    completed.expect("completion won the lease");
                    assert!(recovered.expect("recovery").is_empty());
                    assert_eq!(depths, (0, 0, 0, 0));
                }
                STATUS_FAILED => {
                    assert!(matches!(completed, Err(RedisError::JobLeaseLost { .. })));
                    assert_eq!(recovered.expect("recovery").len(), 1);
                    assert_eq!(depths, (0, 0, 1, 0));
                }
                status => panic!("unexpected authoritative state {status}"),
            }
        })
        .await;
    }
}
