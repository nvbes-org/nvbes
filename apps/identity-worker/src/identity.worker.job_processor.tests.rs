use chrono::{Duration, Utc};
use nvbes_email::{
    EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext,
    EmailTemplate,
};
use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;
use nvbes_redis::worker_queue::{EnqueueJobInput, enqueue_job, get_job};
use redis::AsyncCommands;
use serde_json::json;
use uuid::Uuid;

use super::{
    JOB_LEASE_HEARTBEAT_INTERVAL, WORKER_QUEUES, claim_available_job, failed_job_outcome,
    process_claimed_job,
};
use crate::worker::test_support::app_state;

#[test]
fn identity_worker_queues_exclude_account_and_billing_runtime() {
    assert_eq!(WORKER_QUEUES, [JOB_EMAIL_SUBMIT]);
    for queue in WORKER_QUEUES {
        assert!(!queue.starts_with("billing."), "Billing queue {queue}");
        assert!(!queue.starts_with("account."), "Account queue {queue}");
    }
}

#[test]
fn failed_job_outcome_matches_retry_budget() {
    assert_eq!(failed_job_outcome(1, 3, true), "retry_scheduled");
    assert_eq!(failed_job_outcome(3, 3, true), "dead_letter");
    assert_eq!(failed_job_outcome(1, 3, false), "dead_letter");
}

#[test]
fn lease_heartbeat_precedes_stale_recovery_window() {
    assert!(JOB_LEASE_HEARTBEAT_INTERVAL.as_secs() * 3 < super::super::jobs::STALE_AFTER.as_secs());
}

#[tokio::test]
async fn claim_available_job_returns_none_for_an_empty_owned_queue() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_queue(&state.redis).await;
    assert!(
        claim_available_job(&state, &state.observability)
            .await
            .expect("claim")
            .is_none()
    );
}

#[tokio::test]
async fn processor_completes_a_valid_email_job_end_to_end() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_queue(&state.redis).await;
    let id = enqueue(
        &state.redis,
        JOB_EMAIL_SUBMIT,
        json!(command("reviewer@example.test")),
        3,
    )
    .await;
    let job = claim_available_job(&state, &state.observability)
        .await
        .expect("claim")
        .expect("job");

    process_claimed_job(&state, &state.observability, job)
        .await
        .expect("process");

    assert_eq!(
        get_job(&state.redis, JOB_EMAIL_SUBMIT, id)
            .await
            .expect("get")
            .expect("job")
            .status,
        "succeeded"
    );
    assert!(state.observability.render().contains("outcome=\"success\""));
    cleanup_queue(&state.redis).await;
}

#[tokio::test]
async fn processor_dead_letters_invalid_and_unknown_jobs() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_queue(&state.redis).await;
    for (job_type, payload) in [
        (JOB_EMAIL_SUBMIT, json!({"invalid": true})),
        ("identity.unsupported", json!({})),
    ] {
        let id = enqueue(&state.redis, job_type, payload, 3).await;
        let job = claim_available_job(&state, &state.observability)
            .await
            .expect("claim")
            .expect("job");
        process_claimed_job(&state, &state.observability, job)
            .await
            .expect("process failure");
        assert_eq!(
            get_job(&state.redis, JOB_EMAIL_SUBMIT, id)
                .await
                .expect("get")
                .expect("job")
                .status,
            "dead_letter"
        );
    }
    assert!(
        state
            .observability
            .render()
            .contains("outcome=\"dead_letter\"")
    );
    cleanup_queue(&state.redis).await;
}

#[tokio::test]
async fn processor_schedules_retry_for_transient_email_service_failures() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_queue(&state.redis).await;
    let id = enqueue(
        &state.redis,
        JOB_EMAIL_SUBMIT,
        json!(command("unavailable@example.test")),
        3,
    )
    .await;
    let job = claim_available_job(&state, &state.observability)
        .await
        .expect("claim")
        .expect("job");

    process_claimed_job(&state, &state.observability, job)
        .await
        .expect("record retry");

    let snapshot = get_job(&state.redis, JOB_EMAIL_SUBMIT, id)
        .await
        .expect("get")
        .expect("job");
    assert_eq!(snapshot.status, "failed");
    assert!(
        snapshot
            .last_error
            .expect("safe error")
            .contains("email_service_unavailable")
    );
    assert!(
        state
            .observability
            .render()
            .contains("outcome=\"retry_scheduled\"")
    );
    cleanup_queue(&state.redis).await;
}

fn command(email: &str) -> EmailCommand {
    EmailCommand {
        context: EmailRequestContext {
            request_id: Uuid::new_v4().to_string(),
            correlation_id: Uuid::new_v4().to_string(),
            actor_principal_id: Uuid::new_v4().to_string(),
        },
        producer: "identity-worker".to_string(),
        idempotency_key: EmailIdempotencyKey::new(format!("processor:{}", Uuid::new_v4()))
            .expect("key"),
        recipient: EmailRecipient {
            email: email.to_string(),
            name: Some("Reviewer".to_string()),
        },
        category: EmailCategory::Reminder,
        template: EmailTemplate::AccessReviewReminderV1 {
            reviewer_name: "Reviewer".to_string(),
            campaign_name: "Review".to_string(),
            review_url: "https://account.example.test/access-reviews".to_string(),
            review_due_at: Utc::now() + Duration::days(2),
        },
        deliver_before: Utc::now() + Duration::hours(1),
    }
}

async fn enqueue(
    redis: &nvbes_redis::RedisPool,
    job_type: &str,
    payload: serde_json::Value,
    max_attempts: u32,
) -> Uuid {
    enqueue_job(
        redis,
        EnqueueJobInput {
            queue: JOB_EMAIL_SUBMIT.to_string(),
            job_type: job_type.to_string(),
            payload,
            idempotency_key: None,
            max_attempts,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .expect("enqueue")
}

async fn cleanup_queue(redis: &nvbes_redis::RedisPool) {
    let mut connection = redis.get().await.expect("Redis connection");
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("nvbes:worker_queue:{JOB_EMAIL_SUBMIT}:*"))
        .query_async(&mut *connection)
        .await
        .expect("list queue keys");
    if !keys.is_empty() {
        let _: usize = connection.del(keys).await.expect("delete queue keys");
    }
}
