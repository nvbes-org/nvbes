use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct ProviderRoutingRule {
    id: Uuid,
    priority: i32,
    provider: String,
    country: Option<String>,
    currency: Option<String>,
    payment_method: Option<String>,
    customer_type: Option<String>,
    min_amount_minor: Option<i64>,
    max_amount_minor: Option<i64>,
    fallback_enabled: bool,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub(crate) async fn load_routing_rules(db: &PgPool) -> Result<Vec<ProviderRoutingRule>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, priority, provider::text AS provider, country::text AS country,
          currency::text AS currency, payment_method, customer_type,
          min_amount_minor, max_amount_minor, fallback_enabled, status, created_at, updated_at
        FROM billing_provider_routing_rules
        ORDER BY priority ASC, updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProviderRoutingRule {
            id: row.get("id"),
            priority: row.get("priority"),
            provider: row.get("provider"),
            country: row.get("country"),
            currency: row.get("currency"),
            payment_method: row.get("payment_method"),
            customer_type: row.get("customer_type"),
            min_amount_minor: row.get("min_amount_minor"),
            max_amount_minor: row.get("max_amount_minor"),
            fallback_enabled: row.get("fallback_enabled"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}
