use std::time::Duration;

use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use super::{claim, complete_account, complete_participant, fail};
use crate::types::ClosureParticipant;

#[tokio::test]
#[ignore = "requires NVBES_ACCOUNT_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
async fn closure_claim_and_completion_purge_account_data_atomically() {
    let database_url = std::env::var("NVBES_ACCOUNT_TEST_DATABASE_URL")
        .expect("NVBES_ACCOUNT_TEST_DATABASE_URL is required");
    let db = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("disposable Account database");
    sqlx::migrate!("../account-service-next/migrations")
        .run(&db)
        .await
        .expect("Account migrations");

    let principal_id = Uuid::new_v4();
    let saga_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let requested_at = Utc::now();
    let avatar_object_key = format!("avatars/{principal_id}");
    sqlx::query(
        "INSERT INTO account_profiles (principal_id, firstname, avatar_object_key) VALUES ($1, 'Ada', $2)",
    )
    .bind(principal_id)
    .bind(&avatar_object_key)
    .execute(&db)
    .await
    .expect("profile fixture");
    for table in ["account_preferences", "account_notifications"] {
        sqlx::query(&format!("INSERT INTO {table} (principal_id) VALUES ($1)"))
            .bind(principal_id)
            .execute(&db)
            .await
            .expect("settings fixture");
    }
    sqlx::query(
        "INSERT INTO account_consents (id, principal_id, consent_type, document_version) VALUES ($1, $2, 'privacy', 'v1')",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .execute(&db)
    .await
    .expect("consent fixture");
    sqlx::query(
        "INSERT INTO account_closure_sagas (id, principal_id, status, requested_at, updated_at) VALUES ($1, $2, 'pending', $3, $3)",
    )
    .bind(saga_id)
    .bind(principal_id)
    .bind(requested_at)
    .execute(&db)
    .await
    .expect("saga fixture");
    sqlx::query(
        r#"
        INSERT INTO account_closure_participants (saga_id, participant, ordinal)
        VALUES ($1, 'cloud', 10), ($1, 'billing', 20), ($1, 'identity', 30), ($1, 'account', 40)
        "#,
    )
    .bind(saga_id)
    .execute(&db)
    .await
    .expect("participant fixtures");
    let payload = serde_json::json!({
        "event_id": event_id,
        "event_type": "account.closure.requested.v1",
        "saga_id": saga_id,
        "principal_id": principal_id,
        "requested_at": requested_at,
    });
    sqlx::query(
        r#"
        INSERT INTO account_outbox_events (
          id, aggregate_type, aggregate_id, event_type, payload, occurred_at
        ) VALUES ($1, 'account_closure', $2, 'account.closure.requested.v1', $3, $4)
        "#,
    )
    .bind(event_id)
    .bind(saga_id)
    .bind(payload)
    .bind(requested_at)
    .execute(&db)
    .await
    .expect("closure outbox event");

    let closure = claim(&db, Duration::from_secs(30), 3)
        .await
        .expect("claim")
        .expect("closure event");
    assert_eq!(closure.saga_id, saga_id);
    assert_eq!(closure.participant, ClosureParticipant::Cloud);
    assert_eq!(closure.principal_id, principal_id);
    assert_eq!(
        closure.avatar_object_key.as_deref(),
        Some(avatar_object_key.as_str())
    );
    fail(&db, &closure, true, "cloud_closure_unavailable", 3)
        .await
        .expect("retryable Cloud failure");
    sqlx::query("UPDATE account_outbox_events SET next_attempt_at = NOW() WHERE id = $1")
        .bind(event_id)
        .execute(&db)
        .await
        .expect("retry due now");
    let closure = claim(&db, Duration::from_secs(30), 3)
        .await
        .expect("retry claim")
        .expect("closure retry");
    assert_eq!(closure.participant, ClosureParticipant::Cloud);
    assert_eq!(closure.participant_attempts, 2);
    complete_participant(&db, &closure)
        .await
        .expect("Cloud completion");
    for expected in [ClosureParticipant::Billing, ClosureParticipant::Identity] {
        let closure = claim(&db, Duration::from_secs(30), 3)
            .await
            .expect("claim")
            .expect("closure event");
        assert_eq!(closure.participant, expected);
        complete_participant(&db, &closure)
            .await
            .expect("participant completion");
    }
    let closure = claim(&db, Duration::from_secs(30), 3)
        .await
        .expect("claim")
        .expect("closure event");
    assert_eq!(closure.participant, ClosureParticipant::Account);
    complete_account(&db, &closure)
        .await
        .expect("Account completion");

    let profile_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM account_profiles WHERE principal_id = $1)",
    )
    .bind(principal_id)
    .fetch_one(&db)
    .await
    .expect("profile state");
    assert!(!profile_exists);
    let status: String =
        sqlx::query_scalar("SELECT status FROM account_closure_sagas WHERE id = $1")
            .bind(saga_id)
            .fetch_one(&db)
            .await
            .expect("saga state");
    assert_eq!(status, "completed");
    let published: bool = sqlx::query_scalar(
        "SELECT published_at IS NOT NULL FROM account_outbox_events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_one(&db)
    .await
    .expect("outbox state");
    assert!(published);
}
