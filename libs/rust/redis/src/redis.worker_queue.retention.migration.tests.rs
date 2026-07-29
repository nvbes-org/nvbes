use std::sync::Arc;

use redis::AsyncCommands;
use tokio::sync::Barrier;

use super::retention_migration::{
    RetentionMigrationOutcome, enforce_legacy_terminal_retention, migration_lock_key,
    migration_marker_key,
};
use super::{
    EnqueueJobInput, QueuedJob, claim_next_job, enqueue_job, get_job, keys::*, queue_status,
    retention::REDACTED_TERMINAL_ERROR, test_support::with_isolated_queue,
};
use crate::RedisPool;

const LEGACY_SECRET: &str = "legacy-sensitive-email-token";

fn input(queue: &str, idempotency_key: &str) -> EnqueueJobInput {
    EnqueueJobInput {
        queue: queue.to_string(),
        job_type: "email.send".to_string(),
        payload: serde_json::json!({
            "email": "legacy-private@example.test",
            "token": LEGACY_SECRET,
        }),
        idempotency_key: Some(idempotency_key.to_string()),
        max_attempts: 1,
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

async fn seed_legacy_terminal_job(pool: &RedisPool, mut job: QueuedJob, status: &str) {
    job.status = status.to_string();
    job.claimed_at = None;
    job.lease_token = None;
    job.last_error = Some(format!("legacy failure {LEGACY_SECRET}"));
    job.result = Some(serde_json::json!({ "token": LEGACY_SECRET }));
    let raw = serde_json::to_string(&job).expect("serialize legacy job");
    let mut connection = pool.get().await.expect("redis connection");
    let _: () = connection
        .set(job_key(&job.queue, job.id), raw)
        .await
        .expect("seed legacy terminal job");
    let _: usize = connection
        .zrem(running_key(&job.queue), job.id.to_string())
        .await
        .expect("remove running pointer");
    let _: usize = connection
        .del(lease_key(&job.queue, job.id))
        .await
        .expect("remove lease");
    if status == STATUS_DEAD_LETTER {
        let _: usize = connection
            .sadd(dead_letter_key(&job.queue), job.id.to_string())
            .await
            .expect("seed legacy dead-letter set");
    }
}

#[tokio::test]
async fn status_sweep_scrubs_and_expires_preexisting_terminal_jobs() {
    with_isolated_queue("legacyretention", 4, |redis, queue| async move {
        let success_id = enqueue_job(&redis, input(&queue, "legacy-success"))
            .await
            .expect("enqueue success job");
        let success = claim(&redis, &queue).await;
        seed_legacy_terminal_job(&redis, success, STATUS_SUCCEEDED).await;

        let dead_id = enqueue_job(&redis, input(&queue, "legacy-dead"))
            .await
            .expect("enqueue dead job");
        let dead = claim(&redis, &queue).await;
        seed_legacy_terminal_job(&redis, dead, STATUS_DEAD_LETTER).await;

        let status = queue_status(&redis, &queue)
            .await
            .expect("status triggers retention sweep");
        assert_eq!(
            status
                .iter()
                .find(|entry| entry.status == STATUS_DEAD_LETTER)
                .map(|entry| entry.depth),
            Some(1)
        );

        let success = get_job(&redis, &queue, success_id)
            .await
            .expect("success lookup")
            .expect("success tombstone");
        let dead = get_job(&redis, &queue, dead_id)
            .await
            .expect("dead lookup")
            .expect("dead tombstone");
        for job in [&success, &dead] {
            assert_eq!(job.payload, serde_json::json!({}));
            assert_eq!(job.idempotency_key, None);
            let serialized = serde_json::to_string(job).expect("serialize tombstone");
            assert!(!serialized.contains(LEGACY_SECRET));
            assert!(!serialized.contains("legacy-private@example.test"));
        }
        assert_eq!(
            success.result,
            Some(serde_json::json!({ "redacted": true }))
        );
        assert_eq!(dead.last_error.as_deref(), Some(REDACTED_TERMINAL_ERROR));

        let mut connection = redis.get().await.expect("redis connection");
        for key in [
            job_key(&queue, success_id),
            job_key(&queue, dead_id),
            dedupe_key(&queue, "email.send", "legacy-success"),
            dedupe_key(&queue, "email.send", "legacy-dead"),
            dead_letter_key(&queue),
        ] {
            let ttl: i64 = connection.ttl(&key).await.expect("retained key TTL");
            assert!(ttl > 0, "{key} should have a bounded TTL");
        }
        let index_type: String = redis::cmd("TYPE")
            .arg(dead_letter_key(&queue))
            .query_async(&mut *connection)
            .await
            .expect("dead-letter index type");
        assert_eq!(index_type, "zset");
        let marker_ttl: i64 = connection
            .ttl(migration_marker_key(&queue))
            .await
            .expect("migration marker TTL");
        let lock_exists: bool = connection
            .exists(migration_lock_key(&queue))
            .await
            .expect("migration lock state");
        assert_eq!(marker_ttl, -1, "one-shot marker must be persistent");
        assert!(!lock_exists, "migration owner must release its lock");
    })
    .await;
}

#[tokio::test]
async fn concurrent_replicas_run_exactly_one_idempotent_migration() {
    const JOB_COUNT: usize = 48;
    const REPLICA_COUNT: usize = 12;
    with_isolated_queue("migrationrace", 32, |redis, queue| async move {
        let mut job_ids = Vec::with_capacity(JOB_COUNT);
        for sequence in 0..JOB_COUNT {
            let job_id = enqueue_job(&redis, input(&queue, &format!("legacy-race-{sequence}")))
                .await
                .expect("enqueue legacy job");
            let claimed = claim(&redis, &queue).await;
            seed_legacy_terminal_job(&redis, claimed, STATUS_DEAD_LETTER).await;
            job_ids.push(job_id);
        }

        let barrier = Arc::new(Barrier::new(REPLICA_COUNT));
        let mut replicas = Vec::with_capacity(REPLICA_COUNT);
        for _ in 0..REPLICA_COUNT {
            let pool = redis.clone();
            let queue = queue.clone();
            let barrier = barrier.clone();
            replicas.push(tokio::spawn(async move {
                barrier.wait().await;
                enforce_legacy_terminal_retention(&pool, &queue).await
            }));
        }

        let mut outcomes = Vec::with_capacity(REPLICA_COUNT);
        for replica in replicas {
            outcomes.push(
                replica
                    .await
                    .expect("replica task")
                    .expect("migration outcome"),
            );
        }
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| { **outcome == RetentionMigrationOutcome::Completed })
                .count(),
            1,
            "only the distributed lock owner may complete the migration"
        );
        assert!(outcomes.iter().all(|outcome| matches!(
            outcome,
            RetentionMigrationOutcome::Completed
                | RetentionMigrationOutcome::AlreadyComplete
                | RetentionMigrationOutcome::WaitedForCompletion
        )));

        for job_id in job_ids {
            let stored = get_job(&redis, &queue, job_id)
                .await
                .expect("job lookup")
                .expect("terminal tombstone");
            assert_eq!(stored.payload, serde_json::json!({}));
            assert_eq!(stored.last_error.as_deref(), Some(REDACTED_TERMINAL_ERROR));
        }
        assert_eq!(
            enforce_legacy_terminal_retention(&redis, &queue)
                .await
                .expect("idempotent follow-up"),
            RetentionMigrationOutcome::AlreadyComplete
        );

        let mut connection = redis.get().await.expect("redis connection");
        let marker_ttl: i64 = connection
            .ttl(migration_marker_key(&queue))
            .await
            .expect("persistent marker TTL");
        let lock_exists: bool = connection
            .exists(migration_lock_key(&queue))
            .await
            .expect("released lock");
        assert_eq!(marker_ttl, -1);
        assert!(!lock_exists);
    })
    .await;
}
