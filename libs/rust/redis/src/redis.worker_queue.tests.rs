use super::*;

#[tokio::test]
async fn find_latest_job_returns_most_recent_match() {
    test_support::with_isolated_queue("latest", 2, |redis, queue| async move {
        let older_job_id = enqueue_job(
            &redis,
            EnqueueJobInput {
                queue: queue.clone(),
                job_type: "email.send".to_string(),
                payload: serde_json::json!({
                    "to_email": "old@example.test",
                    "business_type": "verification",
                    "html_body": "<a href=\"https://example.test/verify-result?token=old_token\">",
                    "text_body": "verify-result?token=old_token",
                }),
                idempotency_key: None,
                max_attempts: 3,
                overwrite_terminal: false,
                job_id: None,
            },
        )
        .await
        .expect("older job");

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let newer_job_id = enqueue_job(
            &redis,
            EnqueueJobInput {
                queue: queue.clone(),
                job_type: "email.send".to_string(),
                payload: serde_json::json!({
                    "to_email": "new@example.test",
                    "business_type": "verification",
                    "html_body": "<a href=\"https://example.test/verify-result?token=new_token\">",
                    "text_body": "verify-result?token=new_token",
                }),
                idempotency_key: None,
                max_attempts: 3,
                overwrite_terminal: false,
                job_id: None,
            },
        )
        .await
        .expect("newer job");

        let job = find_latest_job(&redis, &queue, |job| {
            job.job_type == "email.send"
                && job
                    .payload
                    .get("business_type")
                    .and_then(serde_json::Value::as_str)
                    == Some("verification")
                && job
                    .payload
                    .get("to_email")
                    .and_then(serde_json::Value::as_str)
                    == Some("new@example.test")
        })
        .await
        .expect("job lookup")
        .expect("matching job");

        assert_eq!(job.id, newer_job_id);
        assert_ne!(job.id, older_job_id);
    })
    .await;
}
