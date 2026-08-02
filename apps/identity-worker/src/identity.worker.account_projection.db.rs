use std::time::Duration;

use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct ClaimedProjection {
    pub event_id: Uuid,
    pub payload: serde_json::Value,
}

pub struct ProjectionQueueStatus {
    pub pending_depth: i64,
    pub pending_oldest_age_seconds: Option<f64>,
    pub dead_letter_depth: i64,
    pub dead_letter_oldest_age_seconds: Option<f64>,
}

const EVENT_TYPE: &str = "identity.principal.registered.v1";
const MAX_ATTEMPTS: i32 = 12;
const CLAIM_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn claim(db: &PgPool) -> anyhow::Result<Option<ClaimedProjection>> {
    let row = sqlx::query(
        r#"
        WITH candidate AS (
          SELECT id
          FROM identity_outbox_events
          WHERE event_type = $1
            AND dead_lettered_at IS NULL
            AND publish_attempts < $2
            AND next_attempt_at <= NOW()
            AND (claimed_at IS NULL OR claimed_at <= NOW() - make_interval(secs => $3))
          ORDER BY occurred_at, id
          FOR UPDATE SKIP LOCKED
          LIMIT 1
        )
        UPDATE identity_outbox_events event
        SET claimed_at = NOW(),
            publish_attempts = publish_attempts + 1,
            last_error_code = NULL,
            last_error_summary = NULL
        FROM candidate
        WHERE event.id = candidate.id
        RETURNING event.id, event.payload
        "#,
    )
    .bind(EVENT_TYPE)
    .bind(MAX_ATTEMPTS)
    .bind(CLAIM_TIMEOUT.as_secs_f64())
    .fetch_optional(db)
    .await?;
    Ok(row.map(|row| ClaimedProjection {
        event_id: row.get("id"),
        payload: row.get("payload"),
    }))
}

pub async fn complete(db: &PgPool, event_id: Uuid) -> anyhow::Result<()> {
    let result = sqlx::query("DELETE FROM identity_outbox_events WHERE id = $1")
        .bind(event_id)
        .execute(db)
        .await?;
    anyhow::ensure!(
        result.rows_affected() == 1,
        "Account projection outbox state changed"
    );
    Ok(())
}

pub async fn fail(db: &PgPool, event_id: Uuid, retryable: bool, code: &str) -> anyhow::Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE identity_outbox_events
        SET claimed_at = NULL,
            next_attempt_at = CASE
              WHEN $2 AND publish_attempts < $4
                THEN NOW() + make_interval(secs => LEAST(3600, 5 * power(2, publish_attempts))::double precision)
              ELSE next_attempt_at
            END,
            dead_lettered_at = CASE
              WHEN NOT $2 OR publish_attempts >= $4 THEN NOW()
              ELSE NULL
            END,
            last_error_code = $3,
            last_error_summary = $3
        WHERE id = $1
        "#,
    )
    .bind(event_id)
    .bind(retryable)
    .bind(code)
    .bind(MAX_ATTEMPTS)
    .execute(db)
    .await?;
    anyhow::ensure!(
        result.rows_affected() == 1,
        "Account projection outbox state changed"
    );
    Ok(())
}

pub async fn status(db: &PgPool) -> anyhow::Result<ProjectionQueueStatus> {
    let row = sqlx::query(
        r#"
        SELECT
          COUNT(*) FILTER (WHERE dead_lettered_at IS NULL)::BIGINT AS pending_depth,
          EXTRACT(EPOCH FROM (
            NOW() - MIN(occurred_at) FILTER (WHERE dead_lettered_at IS NULL)
          ))::DOUBLE PRECISION AS pending_oldest_age_seconds,
          COUNT(*) FILTER (WHERE dead_lettered_at IS NOT NULL)::BIGINT AS dead_letter_depth,
          EXTRACT(EPOCH FROM (
            NOW() - MIN(dead_lettered_at) FILTER (WHERE dead_lettered_at IS NOT NULL)
          ))::DOUBLE PRECISION AS dead_letter_oldest_age_seconds
        FROM identity_outbox_events
        WHERE event_type = $1
        "#,
    )
    .bind(EVENT_TYPE)
    .fetch_one(db)
    .await?;

    Ok(ProjectionQueueStatus {
        pending_depth: row.get("pending_depth"),
        pending_oldest_age_seconds: row.get("pending_oldest_age_seconds"),
        dead_letter_depth: row.get("dead_letter_depth"),
        dead_letter_oldest_age_seconds: row.get("dead_letter_oldest_age_seconds"),
    })
}
