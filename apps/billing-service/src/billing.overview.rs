use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::BillingResult;

/// V1 account billing snapshot from dedicated billing tables only.
#[derive(Debug, Clone)]
pub struct AccountBillingOverview {
    pub plan_code: String,
    pub status: String,
    pub currency: String,
    pub monthly_price_cents: i64,
    pub customer_email: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub cancel_at_period_end: bool,
    pub current_period_end: Option<DateTime<Utc>>,
}

pub async fn fetch_account_billing_overview(
    db: &PgPool,
    account_id: Uuid,
) -> BillingResult<Option<AccountBillingOverview>> {
    let row = sqlx::query!(
        r#"
        SELECT
          s.plan_code,
          s.status,
          COALESCE(p.currency, 'eur') AS currency,
          COALESCE(NULLIF(s.monthly_price_cents, 0), p.amount_cents, 0) AS monthly_price_cents,
          c.email AS customer_email,
          s.stripe_customer_id,
          s.cancel_at_period_end,
          s.current_period_end
        FROM billing_subscriptions s
        LEFT JOIN billing_plans p ON p.plan_code = s.plan_code
        LEFT JOIN billing_customers c
          ON c.account_id = s.account_id
         AND c.account_type = s.account_type
        WHERE s.account_id = $1
        ORDER BY s.updated_at DESC
        LIMIT 1
        "#,
        account_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| AccountBillingOverview {
        plan_code: r.plan_code,
        status: r.status,
        currency: r.currency.unwrap_or_else(|| "eur".to_string()),
        monthly_price_cents: r.monthly_price_cents.unwrap_or(0),
        customer_email: r.customer_email,
        stripe_customer_id: Some(r.stripe_customer_id),
        cancel_at_period_end: r.cancel_at_period_end,
        current_period_end: r.current_period_end,
    }))
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.overview.tests.rs"]
mod tests;
