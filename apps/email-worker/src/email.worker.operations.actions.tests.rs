use chrono::{Duration, Utc};
use nvbes_email::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient,
    EmailRequestContext, EmailTemplate,
};
use sqlx::PgPool;

use super::{
    ActionError, OperatorAction, apply_suppression, release_suppression, replay_email,
    review_suppression,
};
use crate::{crypto::EmailCrypto, database, operations_privacy, operations_snapshot};

fn crypto() -> EmailCrypto {
    EmailCrypto::new([7; 32], [9; 32])
}

fn operator() -> OperatorAction<'static> {
    OperatorAction {
        actor: "00000000-0000-0000-0000-000000000001",
        reason: "ticket EMAIL-123 approved",
    }
}

async fn accepted_message(pool: &PgPool, recipient: &str) -> uuid::Uuid {
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: "operations-request".to_string(),
            correlation_id: "operations-correlation".to_string(),
            actor_principal_id: "principal-1".to_string(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new(format!("operations-{}", uuid::Uuid::new_v4()))
            .unwrap(),
        recipient: EmailRecipient {
            email: recipient.to_string(),
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
    let receipt = database::accept_command(
        pool,
        &crypto(),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
        .bind(receipt.receipt.message_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn operator_suppression_lifecycle_is_audited_and_readable(pool: PgPool) {
    let email = "suppressed@example.com";
    apply_suppression(&pool, &crypto(), email, "all", operator())
        .await
        .unwrap();

    let snapshot = operations_snapshot::load(&pool, &crypto()).await.unwrap();
    assert_eq!(snapshot.suppressed_email_count, 1);
    assert_eq!(snapshot.recent_suppressions[0].email, email);

    review_suppression(&pool, &crypto(), email, operator())
        .await
        .unwrap();
    let released = release_suppression(&pool, &crypto(), email, operator())
        .await
        .unwrap();
    assert_eq!(released.status, "released");
    let released_again = release_suppression(&pool, &crypto(), email, operator())
        .await
        .unwrap();
    assert_eq!(released_again.status, "already_released");

    let active: bool =
        sqlx::query_scalar("SELECT active FROM email_suppressions WHERE recipient_hash = $1")
            .bind(crypto().recipient_hash(email).as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!active);
    let actions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_operator_actions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(actions, 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_message_can_be_replayed_and_exported_for_privacy(pool: PgPool) {
    let email = "privacy@example.com";
    let message_id = accepted_message(&pool, email).await;
    sqlx::query(
        "UPDATE email_messages SET state = 'failed', terminal_at = clock_timestamp() WHERE id = $1",
    )
    .bind(message_id)
    .execute(&pool)
    .await
    .unwrap();

    let replayed = replay_email(&pool, message_id, operator()).await.unwrap();
    assert_eq!(replayed.status, "queued");
    let state: String = sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
        .bind(message_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "accepted");

    let privacy = operations_privacy::load(&pool, &crypto(), email)
        .await
        .unwrap();
    assert_eq!(privacy.messages.len(), 1);
    assert_eq!(privacy.messages[0].id, message_id.to_string());

    sqlx::query("UPDATE email_messages SET provider_message_id = 'privacy-provider' WHERE id = $1")
        .bind(message_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO email_provider_events (
            id, provider, provider_event_id, sns_message_id, provider_message_id, event_type,
            processed_at, processing_result
        ) VALUES ($1, 'test', 'privacy-event', 'privacy-sns', 'privacy-provider',
                  'email_delivered', clock_timestamp(), 'state_applied')"#,
    )
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();
    let privacy = operations_privacy::load(&pool, &crypto(), email)
        .await
        .unwrap();
    assert_eq!(privacy.provider_events.len(), 1);
    assert_eq!(
        privacy.provider_events[0].processing_result.as_deref(),
        Some("state_applied")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_and_missing_operator_targets_have_distinct_errors(pool: PgPool) {
    assert!(matches!(
        apply_suppression(&pool, &crypto(), "invalid", "all", operator()).await,
        Err(ActionError::Invalid)
    ));
    assert!(matches!(
        apply_suppression(&pool, &crypto(), "valid@example.com", "invalid", operator(),).await,
        Err(ActionError::Invalid)
    ));
    assert!(matches!(
        release_suppression(&pool, &crypto(), "missing@example.com", operator()).await,
        Err(ActionError::NotFound)
    ));
    assert!(matches!(
        review_suppression(&pool, &crypto(), "missing@example.com", operator()).await,
        Err(ActionError::NotFound)
    ));
    assert!(matches!(
        replay_email(&pool, uuid::Uuid::new_v4(), operator()).await,
        Err(ActionError::NotFound)
    ));

    let message_id = accepted_message(&pool, "precondition@example.com").await;
    assert!(matches!(
        replay_email(&pool, message_id, operator()).await,
        Err(ActionError::Precondition)
    ));
}
