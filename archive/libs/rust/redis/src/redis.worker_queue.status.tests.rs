use std::time::Duration;

use super::{EnqueueJobInput, enqueue_job, queue_status, test_support::with_isolated_queue};

#[tokio::test]
async fn queue_status_does_not_require_a_second_pool_connection() {
    with_isolated_queue("status", 1, |redis, queue| async move {
        enqueue_job(
            &redis,
            EnqueueJobInput {
                queue: queue.clone(),
                job_type: "email.send".to_string(),
                payload: serde_json::json!({}),
                idempotency_key: None,
                max_attempts: 3,
                overwrite_terminal: false,
                job_id: None,
            },
        )
        .await
        .expect("enqueue");

        let statuses = tokio::time::timeout(Duration::from_secs(1), queue_status(&redis, &queue))
            .await
            .expect("status lookup must not wait for another connection")
            .expect("queue status");
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].status, "pending");
        assert_eq!(statuses[0].depth, 1);
    })
    .await;
}
