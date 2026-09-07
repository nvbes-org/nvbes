use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::error::BillingResult;

pub async fn record_outbox_event(
    db: impl PgExecutor<'_>,
    event_type: &str,
    aggregate_id: Uuid,
    payload: &Value,
) -> BillingResult<()> {
    sqlx::query(
        r#"
        INSERT INTO billing_outbox (event_type, aggregate_id, payload)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(event_type)
    .bind(aggregate_id)
    .bind(payload)
    .execute(db)
    .await?;

    Ok(())
}
