use chrono::{Duration, Utc};
use nvbes_core::config::AppConfig;
use nvbes_email::EmailTemplate;
use nvbes_product_identity::email::jobs::JOB_EMAIL_SUBMIT;
use nvbes_redis::worker_queue::{EnqueueJobInput, enqueue_job, get_job};
use redis::AsyncCommands;

use super::{
    enqueue_access_review_reminders, reminder_email_command, run_access_review_reminders_if_due,
    run_access_review_schedules_if_due, run_loop_until_shutdown,
};
use crate::grpc_pb::nvbes::enterprise::v1::AccessReviewReminderCandidate;
use crate::worker::test_support::app_state;

#[test]
fn reminder_command_maps_candidate_and_normalizes_the_review_url() {
    let config = AppConfig {
        web_base_url: "https://account.example.test/".to_string(),
        ..AppConfig::default()
    };
    let candidate = candidate((Utc::now() + Duration::days(2)).to_rfc3339());

    let command = reminder_email_command(&config, &candidate).expect("reminder command");

    assert_eq!(command.producer, "identity-worker");
    assert_eq!(command.recipient.email, "reviewer@example.test");
    assert_eq!(command.recipient.name.as_deref(), Some("Reviewer"));
    assert_eq!(command.context.actor_principal_id, "principal-1");
    assert_eq!(command.context.request_id, command.context.correlation_id);
    assert!(command.deliver_before > Utc::now());
    match command.template {
        EmailTemplate::AccessReviewReminderV1 {
            reviewer_name,
            campaign_name,
            review_url,
            review_due_at,
        } => {
            assert_eq!(reviewer_name, "Reviewer");
            assert_eq!(campaign_name, "Quarterly review");
            assert_eq!(review_url, "https://account.example.test/access-reviews");
            assert_eq!(
                review_due_at,
                chrono::DateTime::parse_from_rfc3339(&candidate.due_at)
                    .expect("due at")
                    .with_timezone(&Utc)
            );
        }
        other => panic!("unexpected template: {other:?}"),
    }
}

#[test]
fn reminder_command_rejects_invalid_due_at() {
    let error = reminder_email_command(&AppConfig::default(), &candidate("tomorrow".to_string()))
        .expect_err("invalid due_at");
    assert!(error.to_string().contains("due_at is invalid"));
}

#[tokio::test]
async fn worker_loop_runs_all_due_no_op_integrations_and_honors_shutdown() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_owned_queue(&state.redis).await;
    let observability = state.observability.clone();
    let redis = state.redis.clone();

    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_loop_until_shutdown(state, async {}),
    )
    .await
    .expect("loop shutdown timeout")
    .expect("loop result");

    let rendered = observability.render();
    assert!(rendered.contains("worker_heartbeat"));
    assert!(rendered.contains("integration.email.submit"));
    cleanup_owned_queue(&redis).await;
}

#[tokio::test]
async fn worker_loop_claims_and_dead_letters_an_invalid_owned_job_before_shutdown() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_owned_queue(&state.redis).await;
    let redis = state.redis.clone();
    let id = enqueue_job(
        &redis,
        EnqueueJobInput {
            queue: JOB_EMAIL_SUBMIT.to_string(),
            job_type: JOB_EMAIL_SUBMIT.to_string(),
            payload: serde_json::json!({"invalid": true}),
            idempotency_key: None,
            max_attempts: 3,
            overwrite_terminal: false,
            job_id: None,
        },
    )
    .await
    .expect("enqueue invalid job");

    run_loop_until_shutdown(state, async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    })
    .await
    .expect("loop shutdown");

    assert_eq!(
        get_job(&redis, JOB_EMAIL_SUBMIT, id)
            .await
            .expect("get")
            .expect("job")
            .status,
        "dead_letter"
    );
    cleanup_owned_queue(&redis).await;
}

#[tokio::test]
async fn fresh_enterprise_schedules_are_skipped_without_rpc_calls() {
    let Some(state) = app_state().await else {
        return;
    };
    let mut schedule_last_run = std::time::Instant::now();
    let mut reminder_last_run = std::time::Instant::now();
    run_access_review_schedules_if_due(&state, &mut schedule_last_run)
        .await
        .expect("schedule not due");
    run_access_review_reminders_if_due(&state, &mut reminder_last_run)
        .await
        .expect("reminder not due");
}

#[tokio::test]
async fn enterprise_reminder_candidates_are_enqueued_as_owned_email_jobs() {
    let Some(state) = app_state().await else {
        return;
    };
    cleanup_owned_queue(&state.redis).await;
    let reminder = candidate((Utc::now() + Duration::days(2)).to_rfc3339());

    enqueue_access_review_reminders(&state, &[reminder])
        .await
        .expect("enqueue reminder");

    let job = nvbes_redis::worker_queue::claim_next_job(&state.redis, &[JOB_EMAIL_SUBMIT], 5)
        .await
        .expect("claim reminder")
        .expect("reminder job");
    assert_eq!(job.job_type, JOB_EMAIL_SUBMIT);
    assert_eq!(job.payload["recipient"]["email"], "reviewer@example.test");
    cleanup_owned_queue(&state.redis).await;
}

fn candidate(due_at: String) -> AccessReviewReminderCandidate {
    AccessReviewReminderCandidate {
        tenant_id: "tenant-1".to_string(),
        campaign_id: "campaign-1".to_string(),
        campaign_name: "Quarterly review".to_string(),
        tenant_name: "Tenant".to_string(),
        due_at,
        pending_items: 3,
        recipient_principal_id: "principal-1".to_string(),
        recipient_email: "reviewer@example.test".to_string(),
        recipient_name: "Reviewer".to_string(),
        reminder_kind: "due_soon".to_string(),
    }
}

async fn cleanup_owned_queue(redis: &nvbes_redis::RedisPool) {
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
