use axum::{Json, extract::State};
use serde::Serialize;

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
