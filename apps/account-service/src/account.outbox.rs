use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::AccountResult;

#[derive(Debug, Serialize, FromRow)]
struct OutboxEvent {
    id: Uuid,
    event_type: String,
    aggregate_id: Uuid,
    payload: Value,
    occurred_at: DateTime<Utc>,
    attempts: i16,
}

/// Marks pending Account outbox rows as published.
///
/// V1 has no external sink yet: publication means durable readiness for
/// Platform Operations / future consumers, with attempt accounting.
pub async fn publish_pending(pool: &PgPool, limit: i64) -> AccountResult<usize> {
    let mut tx = pool.begin().await?;
    let events = sqlx::query_as::<_, OutboxEvent>(
        r#"
        SELECT id, event_type, aggregate_id, payload, occurred_at, attempts
        FROM account_outbox
        WHERE published_at IS NULL
        ORDER BY occurred_at ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;

    let count = events.len();
    for event in events {
        tracing::info!(
            outbox_id = %event.id,
            event_type = %event.event_type,
            aggregate_id = %event.aggregate_id,
            attempts = event.attempts + 1,
            "publishing account outbox event"
        );
        sqlx::query(
            "UPDATE account_outbox SET published_at = clock_timestamp(), attempts = attempts + 1 WHERE id = $1",
        )
        .bind(event.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(count)
}
