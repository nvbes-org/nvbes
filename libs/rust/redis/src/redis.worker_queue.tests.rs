use super::*;
use crate::RedisPool;

async fn test_redis_pool() -> RedisPool {
    let mut config = crate::config::RedisConfig::from_env();
    if config.url == "redis://localhost:6379" {
        config.url = "redis://127.0.0.1:6379".to_string();
    }
    config.max_connections = 2;
    crate::connection::create_pool(&config)
        .await
        .expect("redis pool")
}

#[tokio::test]
async fn find_latest_job_returns_most_recent_match() {
    let redis = test_redis_pool().await;
    let queue = format!("worker_queue_test_{}", Uuid::new_v4());

    let older_job_id = enqueue_job(
        &redis,
        &queue,
        "email.send",
        serde_json::json!({
            "to_email": "old@example.test",
            "business_type": "verification",
            "html_body": "<a href=\"https://example.test/verify-result?token=old_token\">",
            "text_body": "verify-result?token=old_token",
        }),
        None,
        3,
        false,
        None,
    )
    .await
    .expect("older job");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let newer_job_id = enqueue_job(
        &redis,
        &queue,
        "email.send",
        serde_json::json!({
            "to_email": "new@example.test",
            "business_type": "verification",
            "html_body": "<a href=\"https://example.test/verify-result?token=new_token\">",
            "text_body": "verify-result?token=new_token",
        }),
        None,
        3,
        false,
        None,
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
}
