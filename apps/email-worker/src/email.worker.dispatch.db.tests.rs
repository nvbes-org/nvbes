use chrono::Duration;
use sqlx::PgPool;

use super::{
    ClaimResult, ClaimedEmail, claim_message, complete_failure, complete_success,
    expire_claim_if_due, suppress_due_messages,
};
use crate::{crypto::EmailCrypto, database, dispatch_attempt::finish_attempt, test_support};

async fn accept(pool: &PgPool, key: &str, email: &str) -> uuid::Uuid {
    let command = test_support::command(key, email);
    let receipt = database::accept_command(
        pool,
        &EmailCrypto::new([7; 32], [9; 32]),
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

async fn claim(pool: &PgPool, id: uuid::Uuid) -> ClaimedEmail {
    match claim_message(pool, id).await.unwrap() {
        ClaimResult::Claimed(claim) => *claim,
        result => panic!("message {id} was not claimable: {result:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn dispatch_attempts_cover_success_retry_expiry_and_stale_leases(pool: PgPool) {
    let first_id = accept(&pool, "dispatch-retry", "retry@example.com").await;
    let first = claim(&pool, first_id).await;
    complete_failure(
        &pool,
        &first,
        "transient_failure",
        "provider_unavailable",
        5,
        Duration::zero(),
    )
    .await
    .unwrap();
    let deferred_attempt: String = sqlx::query_scalar(
        "SELECT outcome::text FROM email_delivery_attempts WHERE lease_token = $1",
    )
    .bind(first.lease_token)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(deferred_attempt, "transient_failure");
    let state: String = sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
        .bind(first_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "deferred");

    let second = claim(&pool, first_id).await;
    complete_failure(
        &pool,
        &second,
        "permanent_failure",
        "invalid_recipient",
        5,
        Duration::minutes(1),
    )
    .await
    .unwrap();
    complete_success(&pool, &second, "stale-completion")
        .await
        .unwrap();

    let expiring_id = accept(&pool, "dispatch-expiry", "expiry@example.com").await;
    let expiring = claim(&pool, expiring_id).await;
    sqlx::query(
        "UPDATE email_messages SET accepted_at = clock_timestamp() - INTERVAL '2 seconds', deliver_before = clock_timestamp() - INTERVAL '1 second' WHERE id = $1",
    )
        .bind(expiring.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(expire_claim_if_due(&pool, &expiring).await.unwrap());
    let expired_outcome: String = sqlx::query_scalar(
        "SELECT outcome::text FROM email_delivery_attempts WHERE lease_token = $1",
    )
    .bind(expiring.lease_token)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(expired_outcome, "expired");

    let stale_id = accept(&pool, "dispatch-stale", "stale@example.com").await;
    let stale = claim(&pool, stale_id).await;
    sqlx::query("UPDATE email_messages SET lease_expires_at = clock_timestamp() - INTERVAL '1 second' WHERE id = $1")
        .bind(stale.id)
        .execute(&pool)
        .await
        .unwrap();
    let reclaimed = claim(&pool, stale_id).await;
    assert_eq!(reclaimed.id, stale.id);
    let stale_outcome: String = sqlx::query_scalar(
        "SELECT outcome::text FROM email_delivery_attempts WHERE lease_token = $1",
    )
    .bind(stale.lease_token)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stale_outcome, "ambiguous_failure");
    complete_success(&pool, &reclaimed, "provider-success")
        .await
        .unwrap();
    let success_outcome: (String, Option<String>) = sqlx::query_as(
        "SELECT outcome::text, provider_message_id FROM email_delivery_attempts WHERE lease_token = $1",
    )
    .bind(reclaimed.lease_token)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(success_outcome.0, "provider_accepted");
    assert_eq!(success_outcome.1.as_deref(), Some("provider-success"));
}

#[sqlx::test(migrations = "./migrations")]
async fn finish_attempt_persists_provider_outcome(pool: PgPool) {
    let id = accept(&pool, "finish-attempt", "finish@example.com").await;
    let claimed = claim(&pool, id).await;
    finish_attempt(
        &pool,
        claimed.lease_token,
        "provider_accepted",
        Some("provider-message-1"),
        None,
    )
    .await
    .unwrap();
    let (outcome, provider_message_id): (String, Option<String>) = sqlx::query_as(
        "SELECT outcome::text, provider_message_id FROM email_delivery_attempts WHERE lease_token = $1",
    )
    .bind(claimed.lease_token)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outcome, "provider_accepted");
    assert_eq!(provider_message_id.as_deref(), Some("provider-message-1"));
}

#[sqlx::test(migrations = "./migrations")]
async fn suppression_sweep_only_blocks_messages_in_scope(pool: PgPool) {
    let optional_id = accept(&pool, "optional-scope", "optional@example.com").await;
    sqlx::query(
        r#"INSERT INTO email_suppressions (
            recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
            scope, reason
        ) SELECT recipient_hash, recipient_ciphertext, recipient_nonce, id, 'optional', 'manual_operator'
          FROM email_messages WHERE id = $1"#,
    )
    .bind(optional_id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(suppress_due_messages(&pool).await.unwrap(), 0);

    let blocked_id = accept(&pool, "all-scope", "blocked-all@example.com").await;
    sqlx::query(
        r#"INSERT INTO email_suppressions (
            recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
            scope, reason
        ) SELECT recipient_hash, recipient_ciphertext, recipient_nonce, id, 'all', 'manual_operator'
          FROM email_messages WHERE id = $1"#,
    )
    .bind(blocked_id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(suppress_due_messages(&pool).await.unwrap(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn claim_message_covers_missing_busy_active_lease_and_deadline(pool: PgPool) {
    assert!(matches!(
        claim_message(&pool, uuid::Uuid::new_v4()).await.unwrap(),
        ClaimResult::Missing
    ));

    let busy_id = accept(&pool, "claim-busy", "claim-busy@example.com").await;
    sqlx::query(
        "UPDATE email_messages SET next_attempt_at = clock_timestamp() + INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(busy_id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        claim_message(&pool, busy_id).await.unwrap(),
        ClaimResult::Busy
    ));

    let leased_id = accept(&pool, "claim-leased", "claim-leased@example.com").await;
    let leased = claim(&pool, leased_id).await;
    assert!(matches!(
        claim_message(&pool, leased_id).await.unwrap(),
        ClaimResult::Settled
    ));
    complete_success(&pool, &leased, "provider-id")
        .await
        .unwrap();
    assert!(matches!(
        claim_message(&pool, leased_id).await.unwrap(),
        ClaimResult::Settled
    ));

    let expired_id = accept(&pool, "claim-deadline", "claim-deadline@example.com").await;
    sqlx::query(
        "UPDATE email_messages SET accepted_at = clock_timestamp() - INTERVAL '2 seconds', deliver_before = clock_timestamp() - INTERVAL '1 second' WHERE id = $1",
    )
    .bind(expired_id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        claim_message(&pool, expired_id).await.unwrap(),
        ClaimResult::Settled
    ));
    let state: String = sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
        .bind(expired_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "expired");

    let suppressed_id = accept(&pool, "claim-suppressed", "claim-suppressed@example.com").await;
    sqlx::query(
        r#"INSERT INTO email_suppressions (
            recipient_hash, recipient_ciphertext, recipient_nonce, recipient_encryption_id,
            scope, reason
        ) SELECT recipient_hash, recipient_ciphertext, recipient_nonce, id, 'all', 'manual_operator'
          FROM email_messages WHERE id = $1"#,
    )
    .bind(suppressed_id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        claim_message(&pool, suppressed_id).await.unwrap(),
        ClaimResult::Settled
    ));
    let suppressed_state: String =
        sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
            .bind(suppressed_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(suppressed_state, "suppressed");
}

#[sqlx::test(migrations = "./migrations")]
async fn complete_failure_expires_when_retry_passes_deadline_and_ignores_stale_leases(
    pool: PgPool,
) {
    let expire_id = accept(&pool, "fail-expire", "fail-expire@example.com").await;
    sqlx::query(
        "UPDATE email_messages SET deliver_before = clock_timestamp() + INTERVAL '30 seconds' WHERE id = $1",
    )
    .bind(expire_id)
    .execute(&pool)
    .await
    .unwrap();
    let claimed = claim(&pool, expire_id).await;
    let resolution = complete_failure(
        &pool,
        &claimed,
        "transient_failure",
        "provider_unavailable",
        5,
        Duration::minutes(5),
    )
    .await
    .unwrap();
    assert!(matches!(
        resolution,
        crate::dispatch_attempt::FailureResolution::Expired
    ));
    let state: String = sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
        .bind(expire_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "expired");

    let stale_id = accept(&pool, "fail-stale", "fail-stale@example.com").await;
    let stale = claim(&pool, stale_id).await;
    complete_success(&pool, &stale, "done").await.unwrap();
    let ignored = complete_failure(
        &pool,
        &stale,
        "transient_failure",
        "provider_unavailable",
        5,
        Duration::minutes(1),
    )
    .await
    .unwrap();
    assert!(matches!(
        ignored,
        crate::dispatch_attempt::FailureResolution::RetryAt(_)
    ));
}
