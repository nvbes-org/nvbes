use std::time::Duration;

use redis::AsyncCommands;

use super::{
    EnqueueJobInput, claim_next_job, enqueue_job,
    keys::{job_key, now_ts, pending_key, running_key},
    recover_stale_jobs, renew_job_lease,
    test_support::with_isolated_queue,
};
use crate::RedisError;

#[tokio::test]
async fn stale_pending_pointer_does_not_invalidate_an_active_lease() {
    with_isolated_queue("stalepointer", 10, |redis, queue| async move {
        enqueue_job(
            &redis,
            EnqueueJobInput {
                queue: queue.clone(),
                job_type: "data.export".to_string(),
                payload: serde_json::json!({}),
                idempotency_key: None,
                max_attempts: 3,
                overwrite_terminal: false,
                job_id: None,
            },
        )
        .await
        .expect("enqueue");
        let claimed = claim_next_job(&redis, &[&queue], 1)
            .await
            .expect("claim")
            .expect("job");
        let mut conn = redis.get().await.expect("redis connection");
        let _: usize = conn
            .rpush(pending_key(&queue), claimed.id.to_string())
            .await
            .expect("inject stale pointer");
        drop(conn);

        assert!(
            claim_next_job(&redis, &[&queue], 1)
                .await
                .expect("empty claim")
                .is_none()
        );
        renew_job_lease(&redis, &claimed)
            .await
            .expect("active lease remains renewable");
    })
    .await;
}

#[tokio::test]
async fn heartbeat_extends_the_lease_and_expired_lease_cannot_be_renewed() {
    with_isolated_queue("lease", 10, |redis, queue| async move {
        enqueue_job(
            &redis,
            EnqueueJobInput {
                queue: queue.clone(),
                job_type: "data.export".to_string(),
                payload: serde_json::json!({}),
                idempotency_key: None,
                max_attempts: 3,
                overwrite_terminal: false,
                job_id: None,
            },
        )
        .await
        .expect("enqueue");
        let claimed = claim_next_job(&redis, &[&queue], 1)
            .await
            .expect("claim")
            .expect("job");

        let mut conn = redis.get().await.expect("redis connection");
        let _: usize = conn
            .zadd(
                running_key(&queue),
                claimed.id.to_string(),
                now_ts() - 1_000,
            )
            .await
            .expect("age lease");
        drop(conn);

        renew_job_lease(&redis, &claimed)
            .await
            .expect("heartbeat renews lease");
        assert!(
            recover_stale_jobs(&redis, &queue, Duration::from_secs(10), Duration::ZERO,)
                .await
                .expect("recovery")
                .is_empty()
        );

        let mut conn = redis.get().await.expect("redis connection");
        let mut expired = claimed.clone();
        expired.claimed_at = Some(now_ts() - 1_000);
        let _: () = conn
            .set(
                job_key(&queue, claimed.id),
                serde_json::to_string(&expired).expect("expired job document"),
            )
            .await
            .expect("age job document");
        let _: usize = conn
            .zadd(
                running_key(&queue),
                claimed.id.to_string(),
                now_ts() - 1_000,
            )
            .await
            .expect("expire lease");
        drop(conn);
        assert_eq!(
            recover_stale_jobs(&redis, &queue, Duration::from_secs(10), Duration::ZERO,)
                .await
                .expect("recovery")
                .len(),
            1
        );
        assert!(matches!(
            renew_job_lease(&redis, &claimed).await,
            Err(RedisError::JobLeaseLost { .. })
        ));
    })
    .await;
}
