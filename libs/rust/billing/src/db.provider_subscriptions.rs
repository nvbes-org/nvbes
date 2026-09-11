use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::provider::ProviderCode;

#[derive(Debug, Clone)]
pub struct ActivateMollieSubscriptionInput<'a> {
    pub workspace_id: Uuid,
    pub plan_id: Uuid,
    pub provider_customer_id: &'a str,
    pub provider_subscription_id: &'a str,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
}

pub async fn activate_mollie_subscription_after_initial_payment_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: ActivateMollieSubscriptionInput<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE subscriptions
        SET plan_id = $2,
            status = 'active',
            billing_provider = 'mollie',
            billing_customer_id = $3,
            billing_subscription_id = $4,
            current_period_start = $5,
            current_period_end = $6,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND (
            billing_subscription_id IS NULL
            OR billing_subscription_id = $4
          )
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.plan_id)
    .bind(input.provider_customer_id)
    .bind(input.provider_subscription_id)
    .bind(input.current_period_start)
    .bind(input.current_period_end)
    .execute(tx.as_mut())
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn workspace_id_for_provider_customer_code_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider: ProviderCode,
    provider_customer_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    super::workspace_id_for_provider_customer_tx(tx, provider.as_str(), provider_customer_id).await
}

pub async fn workspace_id_for_provider_subscription_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider: ProviderCode,
    provider_subscription_id: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    if let Some(workspace_id) = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT bs.workspace_id
        FROM billing_provider_subscriptions bps
        JOIN billing_subscriptions bs ON bs.id = bps.subscription_id
        WHERE bps.provider = $1::billing_provider
          AND bps.provider_subscription_id = $2
          AND bs.workspace_id IS NOT NULL
        ORDER BY bps.primary_for_subscription DESC,
                 bps.updated_at DESC,
                 bps.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(provider.as_str())
    .bind(provider_subscription_id)
    .fetch_optional(tx.as_mut())
    .await?
    {
        return Ok(Some(workspace_id));
    }

    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT workspace_id
        FROM subscriptions
        WHERE billing_provider = $1::billing_provider
          AND billing_subscription_id = $2
        "#,
    )
    .bind(provider.as_str())
    .bind(provider_subscription_id)
    .fetch_optional(tx.as_mut())
    .await
}

#[derive(Debug, Clone)]
pub struct UpsertProviderSubscriptionInput<'a> {
    pub workspace_id: Uuid,
    pub provider: ProviderCode,
    pub provider_customer_id: &'a str,
    pub provider_subscription_id: &'a str,
    pub status: &'a str,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub primary_for_subscription: bool,
    pub metadata: Value,
}

pub async fn upsert_provider_subscription_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: UpsertProviderSubscriptionInput<'_>,
) -> Result<(), sqlx::Error> {
    let fallback_eligible =
        provider_subscription_fallback_eligible(input.status, input.primary_for_subscription);
    sqlx::query(
        r#"
        WITH billing_account AS (
          SELECT id AS billing_account_id, tenant_id
          FROM billing_accounts
          WHERE workspace_id = $1
          LIMIT 1
        ),
        provider_customer AS (
          SELECT pc.id AS provider_customer_row_id,
                 pc.tenant_id,
                 pc.billing_account_id
          FROM billing_provider_customers pc
          JOIN billing_account ba ON ba.billing_account_id = pc.billing_account_id
          WHERE pc.provider = $2::billing_provider
            AND pc.provider_customer_id = $3
            AND pc.status = 'active'
          LIMIT 1
        ),
        existing_subscription AS (
          SELECT bs.id
          FROM billing_subscriptions bs
          JOIN billing_account ba ON ba.billing_account_id = bs.billing_account_id
          WHERE bs.workspace_id = $1
          ORDER BY bs.updated_at DESC, bs.created_at DESC
          LIMIT 1
        ),
        inserted_subscription AS (
          INSERT INTO billing_subscriptions (
            tenant_id,
            workspace_id,
            billing_account_id,
            status,
            current_period_start,
            current_period_end
          )
          SELECT pc.tenant_id,
                 $1,
                 pc.billing_account_id,
                 $5,
                 $6,
                 $7
          FROM provider_customer pc
          WHERE NOT EXISTS (SELECT 1 FROM existing_subscription)
          RETURNING id
        ),
        selected_subscription AS (
          SELECT id FROM existing_subscription
          UNION ALL
          SELECT id FROM inserted_subscription
          LIMIT 1
        ),
        updated_subscription AS (
          UPDATE billing_subscriptions bs
          SET status = $5,
              current_period_start = COALESCE($6, bs.current_period_start),
              current_period_end = COALESCE($7, bs.current_period_end),
              updated_at = NOW()
          FROM selected_subscription ss
          WHERE bs.id = ss.id
          RETURNING bs.id
        ),
        demoted_primary AS (
          UPDATE billing_provider_subscriptions bps
          SET primary_for_subscription = FALSE,
              fallback_eligible = CASE
                WHEN bps.status IN ('active', 'trialing') THEN TRUE
                ELSE bps.fallback_eligible
              END,
              updated_at = NOW()
          FROM updated_subscription us
          WHERE $8
            AND bps.subscription_id = us.id
            AND NOT (
              bps.provider = $2::billing_provider
              AND bps.provider_subscription_id = $4
            )
          RETURNING bps.id
        ),
        demotion_guard AS (
          SELECT COUNT(*) FROM demoted_primary
        )
        INSERT INTO billing_provider_subscriptions (
          tenant_id,
          subscription_id,
          provider_customer_id,
          provider,
          provider_subscription_id,
          status,
          primary_for_subscription,
          fallback_eligible,
          activated_at,
          deactivated_at,
          metadata
        )
        SELECT pc.tenant_id,
               us.id,
               pc.provider_customer_row_id,
               $2::billing_provider,
               $4,
               $5,
               $8,
               $10,
               CASE WHEN $5 IN ('active', 'trialing') THEN NOW() ELSE NULL END,
               CASE WHEN $5 IN ('canceled', 'cancelled') THEN NOW() ELSE NULL END,
               $9
        FROM provider_customer pc
        CROSS JOIN updated_subscription us
        CROSS JOIN demotion_guard
        ON CONFLICT (provider, provider_subscription_id) DO UPDATE
        SET subscription_id = EXCLUDED.subscription_id,
            provider_customer_id = EXCLUDED.provider_customer_id,
            status = EXCLUDED.status,
            primary_for_subscription = EXCLUDED.primary_for_subscription,
            fallback_eligible = EXCLUDED.fallback_eligible,
            activated_at = COALESCE(
              billing_provider_subscriptions.activated_at,
              EXCLUDED.activated_at
            ),
            deactivated_at = EXCLUDED.deactivated_at,
            metadata = EXCLUDED.metadata,
            updated_at = NOW()
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.provider.as_str())
    .bind(input.provider_customer_id)
    .bind(input.provider_subscription_id)
    .bind(input.status)
    .bind(input.current_period_start)
    .bind(input.current_period_end)
    .bind(input.primary_for_subscription)
    .bind(input.metadata)
    .bind(fallback_eligible)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

fn provider_subscription_fallback_eligible(status: &str, primary_for_subscription: bool) -> bool {
    !primary_for_subscription && matches!(status, "active" | "trialing")
}

#[cfg(test)]
mod tests {
    use super::provider_subscription_fallback_eligible;

    #[test]
    fn active_non_primary_provider_subscription_remains_fallback_eligible() {
        assert!(provider_subscription_fallback_eligible("active", false));
        assert!(provider_subscription_fallback_eligible("trialing", false));
    }

    #[test]
    fn primary_provider_subscription_is_never_marked_as_fallback() {
        assert!(!provider_subscription_fallback_eligible("active", true));
        assert!(!provider_subscription_fallback_eligible("trialing", true));
    }

    #[test]
    fn inactive_non_primary_provider_subscription_is_not_fallback_eligible() {
        for status in ["past_due", "incomplete", "canceled", "cancelled", "failed"] {
            assert!(!provider_subscription_fallback_eligible(status, false));
        }
    }
}
