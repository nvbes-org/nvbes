use crate::provider::ProviderCode;
use sqlx::Row;
use uuid::Uuid;

pub struct ProviderSubscriptionContext {
    pub provider_subscription_id: String,
    pub status: Option<String>,
    pub primary_for_subscription: bool,
}

pub async fn provider_subscription_context_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider: ProviderCode,
    provider_subscription_id: &str,
) -> Result<Option<ProviderSubscriptionContext>, sqlx::Error> {
    if let Some(row) = sqlx::query(
        r#"
        SELECT bps.provider_subscription_id,
               bps.status,
               bps.primary_for_subscription
        FROM billing_provider_subscriptions bps
        JOIN billing_subscriptions bs ON bs.id = bps.subscription_id
        WHERE bs.workspace_id = $1
          AND bps.provider = $2::billing_provider
          AND bps.provider_subscription_id = $3
        ORDER BY bps.primary_for_subscription DESC,
                 bps.updated_at DESC,
                 bps.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(provider.as_str())
    .bind(provider_subscription_id)
    .fetch_optional(tx.as_mut())
    .await?
    {
        return Ok(Some(ProviderSubscriptionContext {
            provider_subscription_id: row.get("provider_subscription_id"),
            status: row.get("status"),
            primary_for_subscription: row.get("primary_for_subscription"),
        }));
    }

    let row = sqlx::query(
        r#"
        SELECT billing_subscription_id,
               status::text AS status
        FROM subscriptions
        WHERE workspace_id = $1
          AND billing_provider = $2::billing_provider
          AND billing_subscription_id = $3
        "#,
    )
    .bind(workspace_id)
    .bind(provider.as_str())
    .bind(provider_subscription_id)
    .fetch_optional(tx.as_mut())
    .await?;

    Ok(row.map(|row| ProviderSubscriptionContext {
        provider_subscription_id: row.get("billing_subscription_id"),
        status: row.get("status"),
        primary_for_subscription: true,
    }))
}
