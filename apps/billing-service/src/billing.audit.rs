use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::BillingResult;

pub async fn record_audit_event(
    db: &PgPool,
    account_id: Uuid,
    actor: &str,
    action: &str,
    details: &Value,
) -> BillingResult<()> {
    sqlx::query(
        r#"
        INSERT INTO billing_audit_events (account_id, actor, action, details)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(account_id)
    .bind(actor)
    .bind(action)
    .bind(details)
    .execute(db)
    .await?;

    Ok(())
}
