use crate::provider::{ProviderCode, provider_code};
use sqlx::Row;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProviderRoutingRule {
    pub id: Uuid,
    pub provider: ProviderCode,
    pub fallback_enabled: bool,
}

#[derive(Debug, Error)]
pub enum ProviderRoutingRuleLookupError {
    #[error("provider routing lookup failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("provider routing rule contains unsupported provider: {0}")]
    InvalidProvider(String),
}

pub async fn fetch_provider_routing_rule_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    country: Option<&str>,
    currency: &str,
    payment_method: &str,
    customer_type: &str,
    amount_minor: i64,
) -> Result<Option<ProviderRoutingRule>, ProviderRoutingRuleLookupError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider::text AS provider, fallback_enabled
        FROM billing_provider_routing_rules
        WHERE status = 'active'
          AND (country IS NULL OR country = $1)
          AND (currency IS NULL OR currency = $2)
          AND (payment_method IS NULL OR lower(payment_method) = lower($3))
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
    .fetch_optional(tx.as_mut())
    .await?;

    row.map(|row| {
        let provider_code_value: String = row.get("provider");
        let provider = provider_code(&provider_code_value).ok_or(
            ProviderRoutingRuleLookupError::InvalidProvider(provider_code_value),
        )?;
        Ok(ProviderRoutingRule {
            id: row.get("id"),
            provider,
            fallback_enabled: row.get("fallback_enabled"),
        })
    })
    .transpose()
}
