use chrono::{Duration, Utc};
use nvbes_email::{
    EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext,
    EmailTemplate,
};
use nvbes_redis::worker_queue::QueuedJob;
use serde_json::json;
use uuid::Uuid;

use super::email_command;

#[test]
fn email_command_rejects_malformed_payloads() {
    let error = email_command(&job(json!({"unexpected": true}))).expect_err("malformed command");
    assert_eq!(error.code(), "invalid_email_command_payload");
    assert!(!error.is_retryable());
}

#[test]
fn email_command_rejects_expired_commands() {
    let error = email_command(&job(json!(command(Utc::now() - Duration::seconds(1)))))
        .expect_err("expired command");
    assert_eq!(error.code(), "email_command_expired");
}

#[test]
fn email_command_accepts_a_valid_future_command() {
    let expected = command(Utc::now() + Duration::hours(1));
    let parsed = email_command(&job(json!(expected.clone()))).expect("valid command");
    assert_eq!(parsed, expected);
}

fn command(deliver_before: chrono::DateTime<Utc>) -> EmailCommand {
    EmailCommand {
        context: EmailRequestContext {
            request_id: Uuid::new_v4().to_string(),
            correlation_id: Uuid::new_v4().to_string(),
            actor_principal_id: Uuid::new_v4().to_string(),
        },
        producer: "identity-worker".to_string(),
        idempotency_key: EmailIdempotencyKey::new(format!("test:{}", Uuid::new_v4())).expect("key"),
        recipient: EmailRecipient {
            email: "reviewer@example.test".to_string(),
            name: Some("Reviewer".to_string()),
        },
        category: EmailCategory::Reminder,
        template: EmailTemplate::AccessReviewReminderV1 {
            reviewer_name: "Reviewer".to_string(),
            campaign_name: "Quarterly".to_string(),
            review_url: "https://account.example.test/access-reviews".to_string(),
            review_due_at: Utc::now() + Duration::days(1),
        },
        deliver_before,
    }
}

fn job(payload: serde_json::Value) -> QueuedJob {
    let now = Utc::now().timestamp();
    QueuedJob {
        id: Uuid::new_v4(),
        queue: "integration.email.submit".to_string(),
        job_type: "integration.email.submit".to_string(),
        idempotency_key: None,
        payload,
        max_attempts: 5,
        attempts: 1,
        status: "running".to_string(),
        created_at: now,
        updated_at: now,
        claimed_at: Some(now),
        lease_token: Some(Uuid::new_v4()),
        available_at: now,
        last_error: None,
        result: None,
    }
}
