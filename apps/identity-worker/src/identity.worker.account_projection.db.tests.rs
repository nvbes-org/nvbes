use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{EVENT_TYPE, MAX_ATTEMPTS, claim, complete, fail, status};
use crate::worker::test_support::database_pool;

const AGGREGATE_TYPE: &str = "identity-worker-test";

#[tokio::test]
async fn claim_selects_only_the_oldest_dispatchable_registration() {
    let db = database_pool().await;
    cleanup(&db).await;
    let oldest = insert_event(&db, EVENT_TYPE, -30, 0, None, None).await;
    let newer = insert_event(&db, EVENT_TYPE, -10, 0, None, None).await;
    insert_event(&db, "identity.other.v1", -60, 0, None, None).await;
    insert_event(&db, EVENT_TYPE, -60, 0, Some(60), None).await;
    insert_event(&db, EVENT_TYPE, -60, MAX_ATTEMPTS, None, None).await;
    insert_event(&db, EVENT_TYPE, -60, 0, None, Some(Utc::now())).await;

    let claimed = claim(&db)
        .await
        .expect("claim")
        .expect("dispatchable event");

    assert_eq!(claimed.event_id, oldest);
    assert_eq!(claimed.payload, json!({"event_id": oldest}));
    let row = sqlx::query(
        "SELECT publish_attempts, claimed_at FROM identity_outbox_events WHERE id = $1",
    )
    .bind(oldest)
    .fetch_one(&db)
    .await
    .expect("claimed row");
    assert_eq!(row.get::<i32, _>("publish_attempts"), 1);
    assert!(
        row.get::<Option<chrono::DateTime<Utc>>, _>("claimed_at")
            .is_some()
    );
    assert_eq!(
        claim(&db)
            .await
            .expect("second claim")
            .expect("newer event")
            .event_id,
        newer
    );
    cleanup(&db).await;
}

#[tokio::test]
async fn claim_recovers_stale_claims_but_not_active_leases() {
    let db = database_pool().await;
    cleanup(&db).await;
    let stale = insert_event(&db, EVENT_TYPE, -20, 1, None, None).await;
    let active = insert_event(&db, EVENT_TYPE, -10, 1, None, None).await;
    sqlx::query("UPDATE identity_outbox_events SET claimed_at = NOW() - INTERVAL '10 minutes' WHERE id = $1")
        .bind(stale)
        .execute(&db)
        .await
        .expect("stale claim");
    sqlx::query("UPDATE identity_outbox_events SET claimed_at = NOW() WHERE id = $1")
        .bind(active)
        .execute(&db)
        .await
        .expect("active claim");

    assert_eq!(
        claim(&db)
            .await
            .expect("claim")
            .expect("stale event")
            .event_id,
        stale
    );
    assert!(claim(&db).await.expect("no second claim").is_none());
    cleanup(&db).await;
}

#[tokio::test]
async fn complete_deletes_exactly_one_claimed_event() {
    let db = database_pool().await;
    cleanup(&db).await;
    let event_id = insert_event(&db, EVENT_TYPE, 0, 0, None, None).await;

    complete(&db, event_id).await.expect("complete");

    assert!(!exists(&db, event_id).await);
    let error = complete(&db, event_id)
        .await
        .expect_err("missing row must fail");
    assert!(error.to_string().contains("state changed"));
}

#[tokio::test]
async fn fail_retries_transient_errors_and_dead_letters_permanent_or_exhausted_events() {
    let db = database_pool().await;
    cleanup(&db).await;
    let transient = insert_event(&db, EVENT_TYPE, 0, 1, None, None).await;
    let permanent = insert_event(&db, EVENT_TYPE, 1, 1, None, None).await;
    let exhausted = insert_event(&db, EVENT_TYPE, 2, MAX_ATTEMPTS, None, None).await;

    fail(&db, transient, true, "account_projection_unreachable")
        .await
        .expect("retry");
    fail(&db, permanent, false, "account_projection_rejected")
        .await
        .expect("dead letter");
    fail(&db, exhausted, true, "account_projection_unavailable")
        .await
        .expect("exhausted");

    let transient_row = failure_row(&db, transient).await;
    assert!(transient_row.dead_lettered_at.is_none());
    assert!(transient_row.next_attempt_at > Utc::now());
    assert_eq!(transient_row.code, "account_projection_unreachable");
    assert!(failure_row(&db, permanent).await.dead_lettered_at.is_some());
    assert!(failure_row(&db, exhausted).await.dead_lettered_at.is_some());
    let missing = fail(&db, Uuid::new_v4(), true, "missing")
        .await
        .expect_err("missing row");
    assert!(missing.to_string().contains("state changed"));
    cleanup(&db).await;
}

#[tokio::test]
async fn status_reports_pending_and_dead_letter_depth_and_age() {
    let db = database_pool().await;
    cleanup(&db).await;
    let pending = insert_event(&db, EVENT_TYPE, -120, 0, None, None).await;
    let dead = insert_event(
        &db,
        EVENT_TYPE,
        -60,
        1,
        None,
        Some(Utc::now() - ChronoDuration::seconds(30)),
    )
    .await;
    insert_event(&db, "identity.other.v1", -600, 0, None, None).await;

    let queue = status(&db).await.expect("queue status");

    assert_eq!(queue.pending_depth, 1);
    assert!(queue.pending_oldest_age_seconds.expect("pending age") >= 119.0);
    assert_eq!(queue.dead_letter_depth, 1);
    assert!(queue.dead_letter_oldest_age_seconds.expect("dead age") >= 29.0);
    assert!(exists(&db, pending).await && exists(&db, dead).await);
    cleanup(&db).await;
}

struct FailureRow {
    next_attempt_at: chrono::DateTime<Utc>,
    dead_lettered_at: Option<chrono::DateTime<Utc>>,
    code: String,
}

async fn failure_row(db: &PgPool, event_id: Uuid) -> FailureRow {
    let row = sqlx::query("SELECT next_attempt_at, dead_lettered_at, last_error_code FROM identity_outbox_events WHERE id = $1")
        .bind(event_id)
        .fetch_one(db)
        .await
        .expect("failure row");
    FailureRow {
        next_attempt_at: row.get("next_attempt_at"),
        dead_lettered_at: row.get("dead_lettered_at"),
        code: row.get("last_error_code"),
    }
}

async fn insert_event(
    db: &PgPool,
    event_type: &str,
    occurred_seconds: i64,
    attempts: i32,
    next_attempt_seconds: Option<i64>,
    dead_lettered_at: Option<chrono::DateTime<Utc>>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO identity_outbox_events
           (id, aggregate_type, aggregate_id, event_type, payload, occurred_at, publish_attempts, next_attempt_at, dead_lettered_at)
           VALUES ($1, $2, $3, $4, $5, NOW() + make_interval(secs => $6), $7,
                   NOW() + make_interval(secs => $8), $9)"#,
    )
    .bind(id)
    .bind(AGGREGATE_TYPE)
    .bind(Uuid::new_v4())
    .bind(event_type)
    .bind(json!({"event_id": id}))
    .bind(occurred_seconds as f64)
    .bind(attempts)
    .bind(next_attempt_seconds.unwrap_or(0) as f64)
    .bind(dead_lettered_at)
    .execute(db)
    .await
    .expect("insert outbox event");
    id
}

async fn exists(db: &PgPool, event_id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_outbox_events WHERE id = $1)")
        .bind(event_id)
        .fetch_one(db)
        .await
        .expect("event existence")
}

async fn cleanup(db: &PgPool) {
    sqlx::query("DELETE FROM identity_outbox_events WHERE aggregate_type = $1")
        .bind(AGGREGATE_TYPE)
        .execute(db)
        .await
        .expect("outbox cleanup");
}
