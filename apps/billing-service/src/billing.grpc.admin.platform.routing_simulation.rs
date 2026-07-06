use sqlx::Row;
use tonic::Status;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminBillingRoutingMatchedRule, AdminBillingRoutingSimulationInput,
    AdminBillingRoutingSimulationResult, SimulateAdminBillingRoutingRequest,
};

pub async fn simulate_routing(
    db: &sqlx::PgPool,
    request: SimulateAdminBillingRoutingRequest,
) -> Result<AdminBillingRoutingSimulationResult, Status> {
    if request.amount_minor < 0 {
        return Err(Status::invalid_argument(
            "invalid_routing_amount: Routing simulation amount must be non-negative.",
        ));
    }
    let normalized_country = optional_uppercase(request.country);
    let normalized = AdminBillingRoutingSimulationInput {
        country: normalized_country.clone().unwrap_or_default(),
        currency: request.currency.to_ascii_uppercase(),
        payment_method: request.payment_method.to_ascii_lowercase(),
        customer_type: request.customer_type.to_ascii_lowercase(),
        amount_minor: request.amount_minor,
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
    .bind(normalized_country.as_deref())
    .bind(normalized.currency.as_str())
    .bind(normalized.payment_method.as_str())
    .bind(normalized.customer_type.as_str())
    .bind(normalized.amount_minor)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    let matched_rule = row.map(|row| AdminBillingRoutingMatchedRule {
        id: row.get::<uuid::Uuid, _>("id").to_string(),
        priority: row.get("priority"),
        provider: row.get("provider"),
        fallback_enabled: row.get("fallback_enabled"),
    });
    let outcome = if matched_rule.is_some() {
        "matched"
    } else {
        "no_matching_rule"
    };
    Ok(AdminBillingRoutingSimulationResult {
        input: Some(normalized),
        matched_rule,
        outcome: outcome.to_string(),
    })
}

fn optional_uppercase(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_ascii_uppercase())
    }
}
