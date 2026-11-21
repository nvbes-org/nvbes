use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::error::BillingResult;

pub async fn record_audit_event(
    db: impl PgExecutor<'_>,
    account_id: Uuid,
    actor: &str,
    action: &str,
    details: &Value,
) -> BillingResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO billing_audit_events (account_id, actor, action, details)
        VALUES ($1, $2, $3, $4)
        "#,
        account_id,
        actor,
        action,
        details
    )
    .execute(db)
    .await?;

    Ok(())
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.audit.tests.rs"]
mod tests;
