use chrono::{Duration, Utc};
use nvbes_email::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient,
    EmailRequestContext, EmailTemplate,
};
use sqlx::PgPool;

use super::{TemWebhookEvent, apply_notification};
use crate::{crypto::EmailCrypto, database, dispatch_db};

async fn provider_accepted_message(pool: &PgPool, suffix: &str) {
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: format!("request-{suffix}"),
            correlation_id: format!("correlation-{suffix}"),
            actor_principal_id: "principal-1".to_string(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new(format!("webhook-{suffix}")).unwrap(),
        recipient: EmailRecipient {
            email: format!("webhook-{suffix}@nvbes.fr"),
            name: None,
        },
        category: EmailCategory::AccountSecurity,
        template: EmailTemplate::AccountSecurityV1 {
            event: AccountSecurityEvent::AccountRecovered,
            affected_email: None,
            previous_email: None,
            security_url: None,
        },
        deliver_before: Utc::now() + Duration::hours(1),
    };
    database::accept_command(
        pool,
        &EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    let claim = dispatch_db::claim_one(pool).await.unwrap().unwrap();
    dispatch_db::complete_success(pool, &claim, &format!("provider-{suffix}"))
        .await
        .unwrap();
}

fn event(id: &str, email_id: &str, event_type: &str) -> TemWebhookEvent {
    TemWebhookEvent {
        id: id.to_string(),
        email_id: email_id.to_string(),
        event_type: event_type.to_string(),
        status: Some("sent".to_string()),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn out_of_order_events_increase_knowledge_without_regression(pool: PgPool) {
    provider_accepted_message(&pool, "ordering").await;
    let delivered = apply_notification(
        &pool,
        "sns-delivered",
        &event("event-delivered", "provider-ordering", "email_delivered"),
    )
    .await
    .unwrap();
    assert_eq!(delivered.processing_result, "state_applied");

    let late_queued = apply_notification(
        &pool,
        "sns-queued",
        &event("event-queued", "provider-ordering", "email_queued"),
    )
    .await
    .unwrap();
    assert_eq!(late_queued.processing_result, "state_retained");
    let state: String = sqlx::query_scalar(
        "SELECT state::text FROM email_messages WHERE provider_message_id = 'provider-ordering'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, "delivered");
}

#[sqlx::test(migrations = "./migrations")]
async fn complaint_and_soft_bounce_threshold_create_global_suppressions(pool: PgPool) {
    provider_accepted_message(&pool, "complaint").await;
    apply_notification(
        &pool,
        "sns-spam",
        &event("event-spam", "provider-complaint", "email_spam"),
    )
    .await
    .unwrap();

    provider_accepted_message(&pool, "soft").await;
    for sequence in 1..=3 {
        apply_notification(
            &pool,
            &format!("sns-soft-{sequence}"),
            &event(
                &format!("event-soft-{sequence}"),
                "provider-soft",
                "email_deferred",
            ),
        )
        .await
        .unwrap();
    }

    let suppressions: Vec<(String, bool, i32)> = sqlx::query_as(
        "SELECT reason, active, soft_bounce_count FROM email_suppressions ORDER BY reason",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        suppressions,
        vec![
            ("complaint".to_string(), true, 0),
            ("soft_bounce_threshold".to_string(), true, 3),
        ]
    );
}
