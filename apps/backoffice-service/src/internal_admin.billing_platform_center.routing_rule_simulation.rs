use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub(crate) struct RoutingRuleSimulationInput {
    pub(crate) country: Option<String>,
    pub(crate) currency: String,
    pub(crate) payment_method: String,
    pub(crate) customer_type: String,
    pub(crate) amount_minor: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct RoutingRuleSimulationResult {
    pub(crate) input: NormalizedRoutingRuleSimulationInput,
    pub(crate) matched_rule: Option<MatchedRoutingRule>,
    pub(crate) outcome: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedRoutingRuleSimulationInput {
    pub(crate) country: Option<String>,
    pub(crate) currency: String,
    pub(crate) payment_method: String,
    pub(crate) customer_type: String,
    pub(crate) amount_minor: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatchedRoutingRule {
    pub(crate) id: Uuid,
    pub(crate) priority: i32,
    pub(crate) provider: String,
    pub(crate) fallback_enabled: bool,
}

pub(crate) async fn simulate_routing_rule(
    db: &PgPool,
    input: RoutingRuleSimulationInput,
) -> Result<RoutingRuleSimulationResult, AppError> {
    if input.amount_minor < 0 {
        return Err(AppError::bad_request(
            "invalid_routing_amount",
            "Routing simulation amount must be non-negative.",
        ));
    }
    let normalized = NormalizedRoutingRuleSimulationInput {
        country: input.country.as_deref().map(str::to_ascii_uppercase),
        currency: input.currency.to_ascii_uppercase(),
        payment_method: input.payment_method.to_ascii_lowercase(),
        customer_type: input.customer_type.to_ascii_lowercase(),
        amount_minor: input.amount_minor,
    };
    let row = sqlx::query(
        r#"
        SELECT id, priority, provider::text AS provider, fallback_enabled
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
    .bind(normalized.country.as_deref())
    .bind(normalized.currency.as_str())
    .bind(normalized.payment_method.as_str())
    .bind(normalized.customer_type.as_str())
    .bind(normalized.amount_minor)
    .fetch_optional(db)
    .await?;

    let matched_rule = row.map(|row| MatchedRoutingRule {
        id: row.get("id"),
        priority: row.get("priority"),
        provider: row.get("provider"),
        fallback_enabled: row.get("fallback_enabled"),
    });
    let outcome = if matched_rule.is_some() {
        "matched"
    } else {
        "no_matching_rule"
    };
    Ok(RoutingRuleSimulationResult {
        input: normalized,
        matched_rule,
        outcome,
    })
}
