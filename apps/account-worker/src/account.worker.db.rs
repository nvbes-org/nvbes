use std::time::Duration;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::types::ClaimedProjection;

const EVENT_TYPE: &str = "account.oidc-profile.updated.v1";

pub async fn claim(
    db: &PgPool,
    claim_timeout: Duration,
    max_attempts: i32,
) -> anyhow::Result<Option<ClaimedProjection>> {
    let row = sqlx::query(
        r#"
        WITH candidate AS (
          SELECT id
          FROM account_outbox_events
          WHERE event_type = $1
            AND published_at IS NULL
            AND dead_lettered_at IS NULL
            AND publish_attempts < $2
            AND next_attempt_at <= NOW()
            AND (claimed_at IS NULL OR claimed_at <= NOW() - make_interval(secs => $3))
          ORDER BY occurred_at, id
          FOR UPDATE SKIP LOCKED
          LIMIT 1
        )
        UPDATE account_outbox_events event
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
    .bind(max_attempts)
    .bind(claim_timeout.as_secs_f64())
    .fetch_optional(db)
    .await?;
    Ok(row.map(|row| ClaimedProjection {
        event_id: row.get("id"),
        payload: row.get("payload"),
    }))
}

pub async fn complete(db: &PgPool, event_id: Uuid) -> anyhow::Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET published_at = NOW(), claimed_at = NULL
        WHERE id = $1 AND published_at IS NULL
        "#,
    )
    .bind(event_id)
    .execute(db)
    .await?;
    anyhow::ensure!(result.rows_affected() == 1, "Account outbox state changed");
    Ok(())
}

pub async fn fail(
    db: &PgPool,
    event_id: Uuid,
    retryable: bool,
    code: &str,
    max_attempts: i32,
) -> anyhow::Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE account_outbox_events
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
        WHERE id = $1 AND published_at IS NULL
        "#,
    )
    .bind(event_id)
    .bind(retryable)
    .bind(code)
    .bind(max_attempts)
    .execute(db)
    .await?;
    anyhow::ensure!(result.rows_affected() == 1, "Account outbox state changed");
    Ok(())
}
