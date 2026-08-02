use std::time::Duration;

use sqlx::{PgPool, Row};

use crate::types::{ClaimedClosure, ClosureParticipant};

const EVENT_TYPE: &str = "account.closure.requested.v1";

pub async fn claim(
    db: &PgPool,
    claim_timeout: Duration,
    max_attempts: i32,
) -> anyhow::Result<Option<ClaimedClosure>> {
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT event.id AS event_id,
               event.payload,
               saga.id AS saga_id,
               saga.principal_id,
               profile.avatar_object_key,
               participant.participant,
               participant.attempts
        FROM account_outbox_events event
        JOIN account_closure_sagas saga ON saga.id = event.aggregate_id
        JOIN LATERAL (
          SELECT step.participant, step.attempts
          FROM account_closure_participants step
          WHERE step.saga_id = saga.id AND step.status <> 'completed'
          ORDER BY step.ordinal
          LIMIT 1
        ) participant ON participant.attempts < $2
        LEFT JOIN account_profiles profile ON profile.principal_id = saga.principal_id
        WHERE event.event_type = $1
          AND event.published_at IS NULL
          AND event.dead_lettered_at IS NULL
          AND event.next_attempt_at <= NOW()
          AND (event.claimed_at IS NULL OR event.claimed_at <= NOW() - make_interval(secs => $3))
          AND saga.status IN ('pending', 'dispatching')
        ORDER BY event.occurred_at, event.id
        FOR UPDATE OF event, saga SKIP LOCKED
        LIMIT 1
        "#,
    )
    .bind(EVENT_TYPE)
    .bind(max_attempts)
    .bind(claim_timeout.as_secs_f64())
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };

    let saga_id = row.get("saga_id");
    let participant_name: String = row.get("participant");
    let participant = ClosureParticipant::parse(&participant_name)?;
    let participant_attempts: i32 = row.get::<i32, _>("attempts") + 1;
    let updated = sqlx::query(
        r#"
        UPDATE account_closure_participants
        SET status = 'processing', attempts = attempts + 1,
            started_at = COALESCE(started_at, NOW()), updated_at = NOW(), last_error = NULL
        WHERE saga_id = $1 AND participant = $2 AND status <> 'completed'
        "#,
    )
    .bind(saga_id)
    .bind(participant.as_str())
    .execute(&mut *tx)
    .await?;
    anyhow::ensure!(
        updated.rows_affected() == 1,
        "Account closure participant state changed"
    );

    sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET claimed_at = NOW(), publish_attempts = publish_attempts + 1,
            last_error_code = NULL, last_error_summary = NULL
        WHERE id = $1
        "#,
    )
    .bind(row.get::<uuid::Uuid, _>("event_id"))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE account_closure_sagas SET status = 'dispatching', updated_at = NOW(), last_error = NULL WHERE id = $1",
    )
    .bind(saga_id)
    .execute(&mut *tx)
    .await?;

    let closure = ClaimedClosure {
        event_id: row.get("event_id"),
        saga_id,
        principal_id: row.get("principal_id"),
        avatar_object_key: row.get("avatar_object_key"),
        payload: row.get("payload"),
        participant,
        participant_attempts,
    };
    tx.commit().await?;
    Ok(Some(closure))
}

pub async fn complete_participant(db: &PgPool, closure: &ClaimedClosure) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    complete_participant_tx(&mut tx, closure).await?;
    sqlx::query("UPDATE account_outbox_events SET claimed_at = NULL WHERE id = $1")
        .bind(closure.event_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn complete_account(db: &PgPool, closure: &ClaimedClosure) -> anyhow::Result<()> {
    anyhow::ensure!(
        closure.participant == ClosureParticipant::Account,
        "Account must be the final closure participant"
    );
    let mut tx = db.begin().await?;
    for statement in [
        "DELETE FROM account_consents WHERE principal_id = $1",
        "DELETE FROM account_privacy_exports WHERE principal_id = $1",
        "DELETE FROM account_notifications WHERE principal_id = $1",
        "DELETE FROM account_preferences WHERE principal_id = $1",
        "DELETE FROM account_profiles WHERE principal_id = $1",
    ] {
        sqlx::query(statement)
            .bind(closure.principal_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET dead_lettered_at = NOW(), claimed_at = NULL,
            last_error_code = 'account_closed', last_error_summary = 'Account-owned data was purged'
        WHERE aggregate_id = $1 AND event_type = 'account.oidc-profile.updated.v1'
          AND published_at IS NULL AND dead_lettered_at IS NULL
        "#,
    )
    .bind(closure.principal_id)
    .execute(&mut *tx)
    .await?;
    complete_participant_tx(&mut tx, closure).await?;

    let event = sqlx::query(
        "UPDATE account_outbox_events SET published_at = NOW(), claimed_at = NULL WHERE id = $1 AND aggregate_id = $2 AND published_at IS NULL",
    )
    .bind(closure.event_id)
    .bind(closure.saga_id)
    .execute(&mut *tx)
    .await?;
    anyhow::ensure!(
        event.rows_affected() == 1,
        "Account closure outbox state changed"
    );
    let saga = sqlx::query(
        "UPDATE account_closure_sagas SET status = 'completed', updated_at = NOW(), completed_at = NOW(), last_error = NULL WHERE id = $1 AND principal_id = $2 AND status = 'dispatching'",
    )
    .bind(closure.saga_id)
    .bind(closure.principal_id)
    .execute(&mut *tx)
    .await?;
    anyhow::ensure!(
        saga.rows_affected() == 1,
        "Account closure saga state changed"
    );
    tx.commit().await?;
    Ok(())
}

async fn complete_participant_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    closure: &ClaimedClosure,
) -> anyhow::Result<()> {
    let step = sqlx::query(
        r#"
        UPDATE account_closure_participants
        SET status = 'completed', completed_at = NOW(), updated_at = NOW(), last_error = NULL
        WHERE saga_id = $1 AND participant = $2 AND status = 'processing'
        "#,
    )
    .bind(closure.saga_id)
    .bind(closure.participant.as_str())
    .execute(&mut **tx)
    .await?;
    anyhow::ensure!(
        step.rows_affected() == 1,
        "Account closure participant state changed"
    );
    Ok(())
}

pub async fn fail(
    db: &PgPool,
    closure: &ClaimedClosure,
    retryable: bool,
    code: &str,
    max_attempts: i32,
) -> anyhow::Result<()> {
    let exhausted = !retryable || closure.participant_attempts >= max_attempts;
    let mut tx = db.begin().await?;
    sqlx::query(
        r#"
        UPDATE account_closure_participants
        SET status = 'failed', updated_at = NOW(), last_error = $3
        WHERE saga_id = $1 AND participant = $2 AND status = 'processing'
        "#,
    )
    .bind(closure.saga_id)
    .bind(closure.participant.as_str())
    .bind(code)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE account_outbox_events
        SET claimed_at = NULL,
            next_attempt_at = CASE WHEN $2 THEN next_attempt_at ELSE NOW() + make_interval(secs => LEAST(3600, 5 * power(2, $4))::double precision) END,
            dead_lettered_at = CASE WHEN $2 THEN NOW() ELSE NULL END,
            last_error_code = $3, last_error_summary = $3
        WHERE id = $1 AND published_at IS NULL
        "#,
    )
    .bind(closure.event_id)
    .bind(exhausted)
    .bind(code)
    .bind(closure.participant_attempts)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE account_closure_sagas SET status = CASE WHEN $2 THEN 'failed' ELSE 'dispatching' END, updated_at = NOW(), last_error = $3 WHERE id = $1 AND status = 'dispatching'",
    )
    .bind(closure.saga_id)
    .bind(exhausted)
    .bind(code)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "account.worker.closure.db.tests.rs"]
mod tests;
