use sqlx::PgPool;
use uuid::Uuid;

use super::deterministic_message_id;

#[sqlx::test(migrations = "../identity-service/migrations")]
async fn migration_keeps_the_previous_insert_contract_compatible(pool: PgPool) {
    let mut transaction = pool.begin().await.expect("test transaction");
    let job_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO email_messages (
            job_id,
            business_type,
            recipient_email,
            recipient_hash,
            provider_email_id,
            status
        )
        VALUES ($1, 'legacy_worker', 'recipient@example.test', $2, 'legacy-provider-id', 'sent')
        "#,
    )
    .bind(job_id)
    .bind("0".repeat(64))
    .execute(&mut *transaction)
    .await
    .expect("the pre-migration worker insert shape must remain valid");

    let (message_id, sent_at) =
        sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT message_id, sent_at FROM email_messages WHERE job_id = $1",
        )
        .bind(job_id)
        .fetch_one(&mut *transaction)
        .await
        .expect("legacy delivery should be queryable");

    assert_eq!(message_id, deterministic_message_id(job_id));
    assert!(sent_at.is_some());
    transaction
        .rollback()
        .await
        .expect("test transaction rollback");
}
