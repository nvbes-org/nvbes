use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::provider::ProviderCode;
use crate::stripe_webhook_processing::{
    BillingWebhookProcessingError, BillingWebhookProcessingResult,
};

pub fn provider_subscription_applies_workspace_effects(primary_for_subscription: bool) -> bool {
    primary_for_subscription
}

pub async fn apply_subscription_upsert_workspace_effects(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider_subscription_id: &str,
    status: &str,
    current_period_start: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    object: &Value,
    primary_for_subscription: bool,
) -> BillingWebhookProcessingResult<()> {
    if !provider_subscription_applies_workspace_effects(primary_for_subscription) {
        return Ok(());
    }

    let provider_price_id = object
        .pointer("/items/data/0/price/id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            BillingWebhookProcessingError::bad_request(
                "webhook_missing_price",
                "Subscription is missing price id.",
            )
        })?;
    let plan_id = crate::db::plan_id_for_provider_price_tx(
        tx,
        ProviderCode::Stripe.as_str(),
        provider_price_id,
    )
    .await?
    .ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "unknown_provider_price",
            "Provider price is not mapped to a nvbes plan.",
        )
    })?;

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET plan_id = $2,
            status = $3::subscription_status,
            billing_provider = 'stripe',
            billing_subscription_id = $4,
            current_period_start = $5,
            current_period_end = $6,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .bind(status)
    .bind(provider_subscription_id)
    .bind(current_period_start)
    .bind(current_period_end)
    .execute(tx.as_mut())
    .await?;

    crate::db::project_workspace_plan_tx(tx, workspace_id, plan_id).await?;
    persist_webhook_entitlements(tx, workspace_id, "subscription_upserted", object).await
}

pub async fn apply_subscription_deleted_workspace_effects(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    object: &Value,
    primary_for_subscription: bool,
) -> BillingWebhookProcessingResult<()> {
    if !provider_subscription_applies_workspace_effects(primary_for_subscription) {
        return Ok(());
    }

    let trial_plan_id = crate::db::plan_id_by_code_tx(tx, "trial")
        .await?
        .ok_or_else(|| {
            BillingWebhookProcessingError::not_found("plan_not_found", "Plan not found.")
        })?;
    sqlx::query(
        r#"
        UPDATE subscriptions
        SET plan_id = $2,
            status = 'canceled',
            billing_subscription_id = NULL,
            billing_customer_id = NULL,
            current_period_start = NULL,
            current_period_end = NULL,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(trial_plan_id)
    .execute(tx.as_mut())
    .await?;

    crate::db::project_workspace_plan_tx(tx, workspace_id, trial_plan_id).await?;
    persist_webhook_entitlements(tx, workspace_id, "subscription_deleted", object).await
}

pub async fn apply_invoice_payment_failed_workspace_effects(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider_subscription_id: &str,
    attempt_count: i64,
    object: &Value,
    primary_for_subscription: bool,
) -> BillingWebhookProcessingResult<()> {
    if !provider_subscription_applies_workspace_effects(primary_for_subscription) {
        return Ok(());
    }

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'past_due',
            billing_provider = 'stripe',
            billing_subscription_id = $2,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(provider_subscription_id)
    .execute(tx.as_mut())
    .await?;

    crate::dunning_db::record_payment_failure_tx(tx, workspace_id, attempt_count).await?;
    persist_webhook_entitlements(tx, workspace_id, "payment_failed", object).await
}

pub async fn apply_invoice_payment_succeeded_workspace_effects(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider_subscription_id: &str,
    object: &Value,
    primary_for_subscription: bool,
) -> BillingWebhookProcessingResult<()> {
    if !provider_subscription_applies_workspace_effects(primary_for_subscription) {
        return Ok(());
    }

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'active',
            billing_provider = 'stripe',
            billing_subscription_id = $2,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND status IN ('past_due', 'suspended', 'incomplete')
        "#,
    )
    .bind(workspace_id)
    .bind(provider_subscription_id)
    .execute(tx.as_mut())
    .await?;

    crate::dunning_db::record_payment_success_tx(tx, workspace_id).await?;
    persist_webhook_entitlements(tx, workspace_id, "payment_succeeded", object).await
}

async fn persist_webhook_entitlements(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    reason: &str,
    object: &Value,
) -> BillingWebhookProcessingResult<()> {
    crate::entitlements_db::persist_current_workspace_entitlements_tx(
        tx,
        workspace_id,
        None,
        reason,
        serde_json::json!({ "webhook": object }),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::provider_subscription_applies_workspace_effects;

    #[test]
    fn workspace_effects_apply_only_to_primary_provider_subscription() {
        assert!(provider_subscription_applies_workspace_effects(true));
        assert!(!provider_subscription_applies_workspace_effects(false));
    }
}
