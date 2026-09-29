use crate::{crypto::EmailCrypto, database, test_support};

#[sqlx::test(migrations = "./migrations")]
async fn snapshot_projects_redacted_failures_and_linked_or_orphan_events(pool: sqlx::PgPool) {
    let crypto = EmailCrypto::new([7; 32], [9; 32]);
    let command = test_support::command("snapshot", "snapshot@example.com");
    let receipt = database::accept_command(
        &pool,
        &crypto,
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    let message_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(receipt.receipt.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(
        r#"UPDATE email_messages
           SET state = 'failed', terminal_at = clock_timestamp(), provider_message_id = 'provider-snapshot',
               recipient_ciphertext = NULL, recipient_nonce = NULL, payload_purged_at = clock_timestamp()
           WHERE id = $1"#,
    )
    .bind(message_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO email_provider_events (
            id, provider, provider_event_id, sns_message_id, provider_message_id, event_type
        ) VALUES
          ($1, 'test', 'linked-event', 'linked-sns', 'provider-snapshot', 'email_delivered'),
          ($2, 'test', 'orphan-event', 'orphan-sns', NULL, 'future_event')"#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();

    let snapshot = super::load(&pool, &crypto).await.unwrap();
    assert_eq!(snapshot.failed_message_count_24h, 1);
    assert_eq!(snapshot.recent_failures[0].recipient_email, "redacted");
    assert_eq!(snapshot.recent_unprocessed_events.len(), 2);
    assert!(
        !snapshot.status_distribution.is_empty(),
        "status distribution must reflect persisted messages"
    );
    assert!(
        snapshot
            .status_distribution
            .iter()
            .any(|row| row.status == "failed" && row.message_count >= 1)
    );
    assert!(
        !snapshot.business_type_distribution.is_empty(),
        "business-type distribution must reflect persisted templates"
    );
    assert!(
        snapshot
            .recent_unprocessed_events
            .iter()
            .all(|event| event.email.is_empty() || event.email == "redacted")
    );
}

#[test]
fn recipient_decryption_rejects_invalid_nonce_lengths() {
    let result = super::decrypt_recipient(
        &EmailCrypto::new([7; 32], [9; 32]),
        uuid::Uuid::new_v4(),
        Some(vec![1]),
        Some(vec![0; 11]),
    );
    assert!(result.is_err());
}
