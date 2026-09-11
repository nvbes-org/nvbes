use chrono::Duration;
use sqlx::PgPool;

use super::{
    ClaimResult, ClaimedEmail, claim_message, complete_failure, complete_success,
    expire_claim_if_due, suppress_due_messages,
};
use crate::{crypto::EmailCrypto, database, test_support};

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
