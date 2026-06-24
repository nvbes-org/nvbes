use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct RecentDunningCase {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    status: String,
    policy_state: String,
    opened_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecentDispute {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    status: String,
    currency: String,
    amount_minor: i64,
    created_at: DateTime<Utc>,
}

pub(crate) async fn load_recent_dunning_cases(
    db: &PgPool,
) -> Result<Vec<RecentDunningCase>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT bdc.id, bdc.tenant_id, t.name AS tenant_name, bdc.status,
          bdc.policy_state, bdc.opened_at
        FROM billing_dunning_cases bdc
        JOIN tenants t ON t.id = bdc.tenant_id
        WHERE bdc.status = 'open'
        ORDER BY bdc.opened_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentDunningCase {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            status: row.get("status"),
            policy_state: row.get("policy_state"),
            opened_at: row.get("opened_at"),
        })
        .collect())
}

pub(crate) async fn load_recent_disputes(db: &PgPool) -> Result<Vec<RecentDispute>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT bd.id, bd.tenant_id, t.name AS tenant_name, bd.status,
          bd.currency, bd.amount_minor, bd.created_at
        FROM billing_disputes bd
        JOIN tenants t ON t.id = bd.tenant_id
        ORDER BY bd.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentDispute {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            status: row.get("status"),
            currency: row.get("currency"),
            amount_minor: row.get("amount_minor"),
            created_at: row.get("created_at"),
        })
        .collect())
}
