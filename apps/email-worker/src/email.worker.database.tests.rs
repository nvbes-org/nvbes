use chrono::{Duration, Utc};
use nvbes_email::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient,
    EmailRequestContext, EmailTemplate,
};
use sqlx::PgPool;

use super::{
    AcceptCommandError, accept_command, apply_retention, constant_time_eq, database_now,
    expire_stale_messages, release_suppression_by_message,
};
use crate::{crypto::EmailCrypto, dispatch_db};

fn command(idempotency_key: &str, recipient: &str) -> EmailCommand {
    EmailCommand {
        context: EmailRequestContext {
            request_id: "request-1".to_string(),
            correlation_id: "correlation-1".to_string(),
            actor_principal_id: "principal-1".to_string(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new(idempotency_key).unwrap(),
        recipient: EmailRecipient {
            email: recipient.to_string(),
            name: Some("Ada".to_string()),
        },
        category: EmailCategory::AccountSecurity,
        template: EmailTemplate::AccountSecurityV1 {
            event: AccountSecurityEvent::AccountRecovered,
            affected_email: None,
            previous_email: None,
            security_url: Some("https://identity.nvbes.fr/security".to_string()),
        },
        deliver_before: Utc::now() + Duration::hours(1),
    }
}

async fn accept(pool: &PgPool, command: &EmailCommand) -> nvbes_email::EmailReceipt {
    accept_command(
        pool,
        &EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        command,
    )
    .await
    .unwrap()
    .receipt
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_commands_are_stable_and_conflicts_are_rejected(pool: PgPool) {
    let original = command("security-event-1", "ada@nvbes.fr");
    let first = accept(&pool, &original).await;
    let duplicate = accept(&pool, &original).await;
    assert_eq!(duplicate.message_id, first.message_id);
    assert!(duplicate.duplicate);

    let conflicting = command("security-event-1", "other@nvbes.fr");
    let result = accept_command(
        &pool,
        &EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &conflicting.clone().into_proto(),
        &conflicting,
    )
    .await;
    assert!(matches!(result, Err(AcceptCommandError::Conflict)));
}

#[sqlx::test(migrations = "./migrations")]
async fn active_suppression_settles_current_message_and_release_allows_future_claims(pool: PgPool) {
    let receipt = accept(&pool, &command("security-event-2", "blocked@nvbes.fr")).await;
    let message_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(&receipt.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(
        r#"
        INSERT INTO email_suppressions (
            recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
            scope, reason
        )
        SELECT recipient_hash, recipient_ciphertext, recipient_nonce, id, 'all', 'hard_bounce'
        FROM email_messages WHERE id = $1
        "#,
    )
    .bind(message_id)
    .execute(&pool)
    .await
    .unwrap();

    assert!(matches!(
        dispatch_db::claim_message(&pool, message_id).await.unwrap(),
        dispatch_db::ClaimResult::Settled
    ));
    assert!(
        release_suppression_by_message(&pool, message_id, "operator-1", "address verified")
            .await
            .unwrap()
    );
    let retry = accept(
        &pool,
        &command("security-event-after-release", "blocked@nvbes.fr"),
    )
    .await;
    let retry_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(&retry.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(matches!(
        dispatch_db::claim_message(&pool, retry_id).await.unwrap(),
        dispatch_db::ClaimResult::Claimed(_)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_claims_use_distinct_leases(pool: PgPool) {
    let first = accept(&pool, &command("security-event-3", "first@nvbes.fr")).await;
    let second = accept(&pool, &command("security-event-4", "second@nvbes.fr")).await;
    let first_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(first.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let second_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(second.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let (first, second) = tokio::join!(
        dispatch_db::claim_message(&pool, first_id),
        dispatch_db::claim_message(&pool, second_id)
    );
    let dispatch_db::ClaimResult::Claimed(first) = first.unwrap() else {
        panic!("first email must be claimed");
    };
    let dispatch_db::ClaimResult::Claimed(second) = second.unwrap() else {
        panic!("second email must be claimed");
    };
    assert_ne!(first.id, second.id);
    assert_ne!(first.lease_token, second.lease_token);
}

#[sqlx::test(migrations = "./migrations")]
async fn retention_purges_secrets_before_deleting_the_ledger(pool: PgPool) {
    let receipt = accept(&pool, &command("security-event-5", "retained@nvbes.fr")).await;
    sqlx::query(
        r#"
        UPDATE email_messages
        SET state = 'delivered', terminal_at = clock_timestamp() - INTERVAL '31 days'
        WHERE message_id = $1
        "#,
    )
    .bind(&receipt.message_id)
    .execute(&pool)
    .await
    .unwrap();

    let result = apply_retention(&pool, 30, 400).await.unwrap();
    assert_eq!(result.payloads, 1);
    let payload_is_gone: bool = sqlx::query_scalar(
        "SELECT recipient_ciphertext IS NULL AND template_ciphertext IS NULL FROM email_messages WHERE message_id = $1",
    )
    .bind(&receipt.message_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(payload_is_gone);

    let result = apply_retention(&pool, 1, 1).await.unwrap();
    assert_eq!(result.messages, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn queue_deadline_and_retention_queries_cover_empty_and_populated_ledgers(pool: PgPool) {
    assert!(database_now(&pool).await.unwrap() <= Utc::now() + Duration::seconds(1));
    let receipt = accept(&pool, &command("queue-expiry", "expired@example.com")).await;
    sqlx::query(
        r#"UPDATE email_messages
           SET accepted_at = clock_timestamp() - INTERVAL '2 hours',
               deliver_before = clock_timestamp() - INTERVAL '1 hour'
           WHERE message_id = $1"#,
    )
    .bind(&receipt.message_id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(expire_stale_messages(&pool).await.unwrap(), 1);

    sqlx::query(
        r#"INSERT INTO email_provider_events (
            id, provider, provider_event_id, sns_message_id, event_type, diagnostic,
            processed_at
        ) VALUES ($1, 'test', 'old-event', 'old-sns', 'unknown', '{"detail":"secret"}',
                  clock_timestamp() - INTERVAL '31 days')"#,
    )
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();
    let result = apply_retention(&pool, 30, 400).await.unwrap();
    assert_eq!(result.diagnostics, 1);
    let result = apply_retention(&pool, 1, 1).await.unwrap();
    assert_eq!(result.events, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn acceptance_rejects_missing_wire_payload_and_expired_commands(pool: PgPool) {
    let valid = command("invalid-wire", "wire@example.com");
    let mut missing_recipient = valid.clone().into_proto();
    missing_recipient.recipient = None;
    assert!(matches!(
        accept_command(
            &pool,
            &EmailCrypto::new([7; 32], [9; 32]),
            "nvbes.fr",
            &missing_recipient,
            &valid,
        )
        .await,
        Err(AcceptCommandError::Invalid)
    ));

    let mut expired = command("expired-command", "expired-command@example.com");
    expired.deliver_before = Utc::now() - Duration::seconds(1);
    assert!(matches!(
        accept_command(
            &pool,
            &EmailCrypto::new([7; 32], [9; 32]),
            "nvbes.fr",
            &expired.clone().into_proto(),
            &expired,
        )
        .await,
        Err(AcceptCommandError::Validation(_))
    ));

    assert!(!constant_time_eq(&[1, 2], &[1]));
    assert!(!constant_time_eq(&[1, 2], &[1, 3]));
    assert!(constant_time_eq(&[1, 2], &[1, 2]));
}
