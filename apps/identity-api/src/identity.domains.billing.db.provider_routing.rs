use crate::http::error::AppError;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProviderRoutingRule {
    pub id: Uuid,
    pub provider: String,
    pub fallback_enabled: bool,
}

pub async fn fetch_provider_routing_rule_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    country: Option<&str>,
    currency: &str,
    payment_method: &str,
    customer_type: &str,
    amount_minor: i64,
) -> Result<Option<ProviderRoutingRule>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider::text AS provider, fallback_enabled
        FROM billing_provider_routing_rules
        WHERE status = 'active'
          AND (country IS NULL OR country = $1)
          AND (currency IS NULL OR currency = $2)
          AND (payment_method IS NULL OR payment_method = $3)
          AND (customer_type IS NULL OR lower(customer_type) = lower($4))
          AND (min_amount_minor IS NULL OR min_amount_minor <= $5)
          AND (max_amount_minor IS NULL OR max_amount_minor >= $5)
        ORDER BY
          priority ASC,
          country NULLS LAST,
          currency NULLS LAST,
          payment_method NULLS LAST,
          customer_type NULLS LAST,
          min_amount_minor DESC NULLS LAST,
          max_amount_minor ASC NULLS LAST,
          created_at ASC
        LIMIT 1
        "#,
    )
    .bind(country.map(str::to_ascii_uppercase))
    .bind(currency.to_ascii_uppercase())
    .bind(payment_method)
    .bind(customer_type)
    .bind(amount_minor)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|row| ProviderRoutingRule {
        id: row.get("id"),
        provider: row.get("provider"),
        fallback_enabled: row.get("fallback_enabled"),
    }))
}
