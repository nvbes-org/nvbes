use axum::{Json, extract::State};
use serde::Serialize;
use sqlx::PgPool;

use crate::{app::BillingState, error::BillingResult};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BillingPlanResponse {
    pub plan_code: String,
    pub name: String,
    pub stripe_price_id: String,
    pub currency: String,
    pub amount_cents: i64,
    pub billing_interval: String,
}

pub async fn list_plans_handler(
    State(state): State<BillingState>,
) -> BillingResult<Json<Vec<BillingPlanResponse>>> {
    let plans = sqlx::query_as::<_, BillingPlanResponse>(
        r#"
        SELECT plan_code, name, stripe_price_id, currency, amount_cents, billing_interval
        FROM billing_plans
        WHERE is_active = true
        ORDER BY amount_cents ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(plans))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StripeMappingRow {
    pub plan_code: String,
    pub stripe_price_id: String,
    pub currency: String,
    pub amount_cents: i64,
    pub billing_interval: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct StripeMappingsReport {
    pub plans_checked: usize,
    pub active_plans: usize,
    pub failures: Vec<String>,
}

/// Validates that every billing plan maps to a well-formed Stripe price.
///
/// Rules: `stripe_price_id` must be non-empty and start with `price_`
/// (live and `price_test_*` IDs qualify), unique across plans, amounts
/// non-negative, currency a 3-letter code, interval `month` or `year`,
/// and at least one plan active.
pub async fn check_stripe_mappings(pool: &PgPool) -> anyhow::Result<StripeMappingsReport> {
    let rows = sqlx::query_as::<_, StripeMappingRow>(
        r#"
        SELECT plan_code, stripe_price_id, currency, amount_cents, billing_interval, is_active
        FROM billing_plans
        ORDER BY plan_code ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut failures = Vec::new();
    let mut seen_price_ids = std::collections::HashSet::new();
    for row in &rows {
        if row.plan_code.trim().is_empty() {
            failures.push("billing_plans: empty plan_code".to_string());
        }
        if row.stripe_price_id.trim().is_empty() {
            failures.push(format!("{}: empty stripe_price_id", row.plan_code));
        } else if !row.stripe_price_id.starts_with("price_") {
            failures.push(format!(
                "{}: stripe_price_id {:?} must start with \"price_\"",
                row.plan_code, row.stripe_price_id
            ));
        }
        if !seen_price_ids.insert(row.stripe_price_id.clone()) {
            failures.push(format!(
                "{}: duplicate stripe_price_id {:?}",
                row.plan_code, row.stripe_price_id
            ));
        }
        if row.amount_cents < 0 {
            failures.push(format!(
                "{}: negative amount_cents {}",
                row.plan_code, row.amount_cents
            ));
        }
        if row.currency.len() != 3 || !row.currency.bytes().all(|b| b.is_ascii_lowercase()) {
            failures.push(format!(
                "{}: currency {:?} must be a 3-letter lowercase code",
                row.plan_code, row.currency
            ));
        }
        if row.billing_interval != "month" && row.billing_interval != "year" {
            failures.push(format!(
                "{}: billing_interval {:?} must be \"month\" or \"year\"",
                row.plan_code, row.billing_interval
            ));
        }
    }
    let active_plans = rows.iter().filter(|row| row.is_active).count();
    if rows.is_empty() {
        failures.push("billing_plans: no plans defined".to_string());
    }
    if active_plans == 0 {
        failures.push("billing_plans: no active plan".to_string());
    }

    Ok(StripeMappingsReport {
        plans_checked: rows.len(),
        active_plans,
        failures,
    })
}
