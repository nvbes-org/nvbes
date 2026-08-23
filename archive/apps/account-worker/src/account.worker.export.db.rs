use std::time::Duration;

use nvbes_product_account::export_event::AccountExportFragmentV1;
use serde_json::Value;
use sqlx::{PgPool, Row};

use crate::types::{ClaimedExport, ExportParticipant};

const EVENT_TYPE: &str = "account.export.requested.v1";

pub async fn claim(
    db: &PgPool,
    claim_timeout: Duration,
    max_attempts: i32,
) -> anyhow::Result<Option<ClaimedExport>> {
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"SELECT event.id AS event_id, event.payload, export.id AS export_id,
                  export.principal_id, step.participant, step.attempts
           FROM account_outbox_events event
           JOIN account_privacy_exports export ON export.id = event.aggregate_id
           JOIN LATERAL (
             SELECT participant, attempts FROM account_export_participants
             WHERE export_id = export.id AND status <> 'completed'
             ORDER BY ordinal LIMIT 1
           ) step ON step.attempts < $2
           WHERE event.event_type = $1 AND event.published_at IS NULL
             AND event.dead_lettered_at IS NULL AND event.next_attempt_at <= NOW()
             AND (event.claimed_at IS NULL OR event.claimed_at <= NOW() - make_interval(secs => $3))
             AND export.status IN ('pending', 'processing')
           ORDER BY event.occurred_at, event.id
           FOR UPDATE OF event, export SKIP LOCKED LIMIT 1"#,
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
    let export_id = row.get("export_id");
    let participant_name: String = row.get("participant");
    let participant = ExportParticipant::parse(&participant_name)?;
    let participant_attempts = row.get::<i32, _>("attempts") + 1;
    let updated = sqlx::query(
        r#"UPDATE account_export_participants SET status = 'processing', attempts = attempts + 1,
             started_at = COALESCE(started_at, NOW()), updated_at = NOW(), last_error = NULL
           WHERE export_id = $1 AND participant = $2 AND status <> 'completed'"#,
    )
    .bind(export_id)
    .bind(participant.as_str())
    .execute(&mut *tx)
    .await?;
    anyhow::ensure!(
        updated.rows_affected() == 1,
        "Account export participant state changed"
    );
    sqlx::query(
        "UPDATE account_outbox_events SET claimed_at = NOW(), publish_attempts = publish_attempts + 1, last_error_code = NULL, last_error_summary = NULL WHERE id = $1",
    )
    .bind(row.get::<uuid::Uuid, _>("event_id"))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE account_privacy_exports SET status = 'processing', updated_at = NOW(), last_error = NULL WHERE id = $1",
    )
    .bind(export_id)
    .execute(&mut *tx)
    .await?;
    let claimed = ClaimedExport {
        event_id: row.get("event_id"),
        export_id,
        principal_id: row.get("principal_id"),
        payload: row.get("payload"),
        participant,
        participant_attempts,
    };
    tx.commit().await?;
    Ok(Some(claimed))
}

pub async fn build_account_fragment(
    db: &PgPool,
    principal_id: uuid::Uuid,
) -> anyhow::Result<Value> {
    let data = sqlx::query_scalar::<_, Value>(
        r#"SELECT jsonb_build_object(
          'profile', (SELECT to_jsonb(p) - 'avatar_object_key' FROM account_profiles p WHERE principal_id = $1),
          'preferences', (SELECT to_jsonb(p) - 'principal_id' FROM account_preferences p WHERE principal_id = $1),
          'notifications', (SELECT to_jsonb(n) - 'principal_id' FROM account_notifications n WHERE principal_id = $1),
          'consents', COALESCE((SELECT jsonb_agg(to_jsonb(c) ORDER BY granted_at DESC, id DESC)
             FROM account_consents c WHERE principal_id = $1), '[]'::jsonb),
          'closure_requests', COALESCE((SELECT jsonb_agg(jsonb_build_object(
             'id', id, 'status', status, 'requested_at', requested_at,
             'updated_at', updated_at, 'completed_at', completed_at
           ) ORDER BY requested_at DESC, id DESC)
             FROM account_closure_sagas WHERE principal_id = $1), '[]'::jsonb)
        )"#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    Ok(serde_json::to_value(AccountExportFragmentV1::new(
        "account",
        principal_id,
        data,
    ))?)
}

pub async fn complete(db: &PgPool, export: &ClaimedExport, fragment: Value) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    let step = sqlx::query(
        r#"UPDATE account_export_participants SET status = 'completed', fragment = $3,
             completed_at = NOW(), updated_at = NOW(), last_error = NULL
           WHERE export_id = $1 AND participant = $2 AND status = 'processing'"#,
    )
    .bind(export.export_id)
    .bind(export.participant.as_str())
    .bind(fragment)
    .execute(&mut *tx)
    .await?;
    anyhow::ensure!(
        step.rows_affected() == 1,
        "Account export participant state changed"
    );
    if export.participant == ExportParticipant::Account {
        finalize(&mut tx, export).await?;
    } else {
        sqlx::query("UPDATE account_outbox_events SET claimed_at = NULL WHERE id = $1")
            .bind(export.event_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn finalize(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    export: &ClaimedExport,
) -> anyhow::Result<()> {
    let document = sqlx::query_scalar::<_, Value>(
        r#"SELECT jsonb_build_object(
          'schema_version', 'nvbes-account-export.v1', 'export_id', $1,
          'principal_id', $2, 'generated_at', NOW(),
          'products', jsonb_object_agg(participant, fragment -> 'data' ORDER BY ordinal),
          'coverage', jsonb_build_object('secrets_excluded', true, 'file_content_included', false)
        ) FROM account_export_participants WHERE export_id = $1"#,
    )
    .bind(export.export_id)
    .bind(export.principal_id)
    .fetch_one(&mut **tx)
    .await?;
    let result = sqlx::query(
        r#"UPDATE account_privacy_exports SET status = 'completed', document = $2,
             completed_at = NOW(), expires_at = NOW() + INTERVAL '24 hours', updated_at = NOW(), last_error = NULL
           WHERE id = $1 AND status = 'processing'"#,
    )
    .bind(export.export_id)
    .bind(document)
    .execute(&mut **tx)
    .await?;
    anyhow::ensure!(result.rows_affected() == 1, "Account export state changed");
    sqlx::query(
        "UPDATE account_outbox_events SET published_at = NOW(), claimed_at = NULL WHERE id = $1",
    )
    .bind(export.event_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn fail(
    db: &PgPool,
    export: &ClaimedExport,
    retryable: bool,
    code: &str,
    max_attempts: i32,
) -> anyhow::Result<()> {
    let exhausted = !retryable || export.participant_attempts >= max_attempts;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE account_export_participants SET status = 'failed', updated_at = NOW(), last_error = $3 WHERE export_id = $1 AND participant = $2 AND status = 'processing'")
        .bind(export.export_id).bind(export.participant.as_str()).bind(code).execute(&mut *tx).await?;
    sqlx::query(
        r#"UPDATE account_outbox_events SET claimed_at = NULL,
             next_attempt_at = CASE WHEN $2 THEN next_attempt_at ELSE NOW() + make_interval(secs => LEAST(3600, 5 * power(2, $4))::double precision) END,
             dead_lettered_at = CASE WHEN $2 THEN NOW() ELSE NULL END,
             last_error_code = $3, last_error_summary = $3 WHERE id = $1 AND published_at IS NULL"#,
    )
    .bind(export.event_id).bind(exhausted).bind(code).bind(export.participant_attempts)
    .execute(&mut *tx).await?;
    sqlx::query("UPDATE account_privacy_exports SET status = CASE WHEN $2 THEN 'failed' ELSE 'processing' END, updated_at = NOW(), last_error = $3 WHERE id = $1")
        .bind(export.export_id).bind(exhausted).bind(code).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "account.worker.export.db.tests.rs"]
mod tests;
