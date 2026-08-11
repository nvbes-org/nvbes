use nvbes_observability::metrics::HttpMetrics;
use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;
use nvbes_redis::worker_queue::{EnqueueJobInput, enqueue_job, get_job};
use redis::AsyncCommands;
use serde_json::json;
use uuid::Uuid;

use super::{
    STALE_AFTER, claim_next_job, mark_job_failed, mark_job_succeeded, recover_stale_jobs,
    renew_job_lease, should_retry_job,
};
use crate::worker::{job_failure::JobExecutionError, test_support::redis_pool};

#[test]
fn identity_worker_retries_only_transient_owned_jobs() {
    let transient = JobExecutionError::transient("provider_unavailable", "Provider is unavailable");
    let permanent = JobExecutionError::permanent("invalid_payload", "Job payload is invalid");

    assert!(should_retry_job(JOB_EMAIL_SUBMIT, &transient));
    assert!(!should_retry_job(JOB_EMAIL_SUBMIT, &permanent));

    for job_type in [
        concat!("billing.", "stripe.webhook.process"),
        concat!("billing.", "mollie.webhook.process"),
        concat!("billing.", "email.send"),
    ] {
        assert!(
            !should_retry_job(job_type, &transient),
            "foreign job {job_type}"
        );
    }
}

#[tokio::test]
async fn queue_wrappers_cover_success_retry_dead_letter_and_lease_renewal() {
    let redis = redis_pool().await;
    let queue = unique_queue("lifecycle");

    let success_id = enqueue(&redis, &queue, 3).await;
    let success = claim_next_job(&redis, &[&queue])
        .await
        .expect("claim")
        .expect("success job");
    assert_eq!(success.id, success_id);
    renew_job_lease(&redis, &success)
        .await
        .expect("renew lease");
    mark_job_succeeded(&redis, &success, json!({"accepted": true}))
        .await
        .expect("succeed");
    assert_eq!(
        get_job(&redis, &queue, success_id)
            .await
            .expect("get")
            .expect("job")
            .status,
        "succeeded"
    );

    let retry_id = enqueue(&redis, &queue, 3).await;
    let retry = claim_next_job(&redis, &[&queue])
        .await
        .expect("claim retry")
        .expect("retry job");
    mark_job_failed(&redis, &retry, "temporary", true)
        .await
        .expect("retry failure");
    let retried = get_job(&redis, &queue, retry_id)
        .await
        .expect("get")
        .expect("retry snapshot");
    assert_eq!(retried.status, "failed");
    assert!(retried.available_at >= chrono::Utc::now().timestamp() + 59);

    let dead_id = enqueue(&redis, &queue, 1).await;
    let dead = claim_next_job(&redis, &[&queue])
        .await
        .expect("claim dead")
        .expect("dead job");
    mark_job_failed(&redis, &dead, "permanent", false)
        .await
        .expect("dead letter");
    assert_eq!(
        get_job(&redis, &queue, dead_id)
            .await
            .expect("get")
            .expect("dead snapshot")
            .status,
        "dead_letter"
    );
    cleanup_queue(&redis, &queue).await;
}

#[tokio::test]
async fn stale_job_wrapper_recovers_a_claim_whose_lease_expired() {
    let redis = redis_pool().await;
    let queue = unique_queue("stale");
    let id = enqueue(&redis, &queue, 3).await;
    let mut job = claim_next_job(&redis, &[&queue])
        .await
        .expect("claim")
        .expect("job");
    let stale_at = chrono::Utc::now().timestamp() - STALE_AFTER.as_secs() as i64 - 1;
    job.claimed_at = Some(stale_at);
    job.updated_at = stale_at;
    let mut connection = redis.get().await.expect("Redis connection");
    let prefix = format!("nvbes:worker_queue:{queue}");
    let _: () = connection
        .set(
            format!("{prefix}:job:{id}"),
            serde_json::to_string(&job).expect("job JSON"),
        )
        .await
        .expect("write stale job");
    let _: () = connection
        .zadd(format!("{prefix}:running"), id.to_string(), stale_at)
        .await
        .expect("write stale score");
    drop(connection);

    recover_stale_jobs(&redis, &queue, &HttpMetrics::default())
        .await
        .expect("recover");

    let recovered = get_job(&redis, &queue, id)
        .await
        .expect("get")
        .expect("recovered job");
    assert_eq!(recovered.status, "failed");
    assert_eq!(recovered.last_error.as_deref(), Some("job lease expired"));
    cleanup_queue(&redis, &queue).await;
}

async fn enqueue(redis: &nvbes_redis::RedisPool, queue: &str, max_attempts: u32) -> Uuid {
    enqueue_job(
        redis,
        EnqueueJobInput {
            queue: queue.to_string(),
            job_type: JOB_EMAIL_SUBMIT.to_string(),
            payload: json!({"sequence": Uuid::new_v4()}),
            idempotency_key: None,
            max_attempts,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .expect("enqueue")
}

fn unique_queue(label: &str) -> String {
    format!("identity_worker_{label}_{}", Uuid::new_v4())
}

async fn cleanup_queue(redis: &nvbes_redis::RedisPool, queue: &str) {
    let mut connection = redis.get().await.expect("Redis connection");
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("nvbes:worker_queue:{queue}:*"))
        .query_async(&mut *connection)
        .await
        .expect("list test queue keys");
    if !keys.is_empty() {
        let _: usize = connection.del(keys).await.expect("delete test queue keys");
    }
}
