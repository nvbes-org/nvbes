use std::time::Duration;

use anyhow::Context;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::types::ClaimedClosure;

const OWNED_DATA_TABLES: [&str; 5] = [
    "account_consents",
    "account_privacy_exports",
    "account_notifications",
    "account_preferences",
    "account_profiles",
];

pub async fn claim_next_closure(
    db: &PgPool,
    retry_interval: Duration,
    claim_timeout: Duration,
    max_attempts: i32,
) -> anyhow::Result<Option<ClaimedClosure>> {
    let mut tx = db.begin().await.context("begin closure claim")?;
    let row = sqlx::query(
        r#"
        SELECT
          e.id AS event_id,
          e.aggregate_id AS saga_id,
          e.publish_attempts,
          s.principal_id,
          s.requested_at,
          p.avatar_object_key
        FROM account_outbox_events e
        INNER JOIN account_closure_sagas s ON s.id = e.aggregate_id
        LEFT JOIN account_profiles p ON p.principal_id = s.principal_id
        WHERE e.aggregate_type = 'account_closure'
          AND e.event_type = 'account.closure.requested'
          AND e.published_at IS NULL
          AND e.publish_attempts < $1
          AND (
            (
              s.status = 'pending'
              AND (
                s.last_error IS NULL
                OR s.updated_at <= NOW() - make_interval(secs => $2::double precision)
              )
            )
            OR (
              s.status = 'dispatching'
              AND s.updated_at <= NOW() - make_interval(secs => $3::double precision)
            )
          )
        ORDER BY e.occurred_at, e.id
        FOR UPDATE OF e, s SKIP LOCKED
        LIMIT 1
        "#,
    )
    .bind(max_attempts)
    .bind(duration_seconds(retry_interval))
    .bind(duration_seconds(claim_timeout))
    .fetch_optional(&mut *tx)
    .await
    .context("select closure outbox event")?;

    let Some(row) = row else {
        tx.commit().await.context("commit empty closure claim")?;
        return Ok(None);
    };

    let event_id: Uuid = row.try_get("event_id")?;
    let saga_id: Uuid = row.try_get("saga_id")?;
    let previous_attempts: i32 = row.try_get("publish_attempts")?;

    sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET publish_attempts = publish_attempts + 1
        WHERE id = $1
          AND published_at IS NULL
        "#,
    )
    .bind(event_id)
    .execute(&mut *tx)
    .await
    .context("increment closure publish attempt")?;

    sqlx::query(
        r#"
        UPDATE account_closure_sagas
        SET status = 'dispatching',
            updated_at = NOW(),
            last_error = NULL
        WHERE id = $1
        "#,
    )
    .bind(saga_id)
    .execute(&mut *tx)
    .await
    .context("mark closure saga dispatching")?;

    let claimed = ClaimedClosure {
        event_id,
        saga_id,
        principal_id: row.try_get("principal_id")?,
        requested_at: row.try_get("requested_at")?,
        avatar_object_key: row.try_get("avatar_object_key")?,
        attempt: previous_attempts + 1,
    };
    tx.commit().await.context("commit closure claim")?;
    Ok(Some(claimed))
}

pub async fn record_closure_failure(
    db: &PgPool,
    saga_id: Uuid,
    retryable: bool,
    safe_error: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE account_closure_sagas
        SET status = CASE WHEN $2 THEN 'pending' ELSE 'failed' END,
            updated_at = NOW(),
            last_error = $3
        WHERE id = $1
          AND status = 'dispatching'
        "#,
    )
    .bind(saga_id)
    .bind(retryable)
    .bind(safe_error)
    .execute(db)
    .await
    .context("record closure failure")?;
    Ok(())
}

pub async fn complete_closure(db: &PgPool, event: &ClaimedClosure) -> anyhow::Result<()> {
    let mut tx = db.begin().await.context("begin closure completion")?;

    for table in OWNED_DATA_TABLES {
        sqlx::query(&format!("DELETE FROM {table} WHERE principal_id = $1"))
            .bind(event.principal_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("purge {table}"))?;
    }

    let saga = sqlx::query(
        r#"
        UPDATE account_closure_sagas
        SET status = 'completed',
            completed_at = NOW(),
            updated_at = NOW(),
            last_error = NULL
        WHERE id = $1
          AND principal_id = $2
          AND status = 'dispatching'
        "#,
    )
    .bind(event.saga_id)
    .bind(event.principal_id)
    .execute(&mut *tx)
    .await
    .context("complete closure saga")?;
    anyhow::ensure!(saga.rows_affected() == 1, "closure saga state changed");

    let outbox = sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET published_at = NOW()
        WHERE id = $1
          AND aggregate_id = $2
          AND event_type = 'account.closure.requested'
          AND published_at IS NULL
        "#,
    )
    .bind(event.event_id)
    .bind(event.saga_id)
    .execute(&mut *tx)
    .await
    .context("publish closure outbox event")?;
    anyhow::ensure!(outbox.rows_affected() == 1, "closure outbox state changed");

    tx.commit().await.context("commit closure completion")?;
    Ok(())
}

fn duration_seconds(duration: Duration) -> f64 {
    duration.as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::OWNED_DATA_TABLES;

    #[test]
    fn closure_purges_only_account_owned_personal_data() {
        assert_eq!(
            OWNED_DATA_TABLES,
            [
                "account_consents",
                "account_privacy_exports",
                "account_notifications",
                "account_preferences",
                "account_profiles",
            ]
        );
        assert!(
            OWNED_DATA_TABLES
                .iter()
                .all(|table| table.starts_with("account_"))
        );
    }
}
