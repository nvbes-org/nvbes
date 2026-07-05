use serde_json::Value;
use uuid::Uuid;

use crate::provider::ProviderCode;
use crate::stripe_webhook_persistence::{
    persist_stripe_payment_method_if_present, persist_stripe_subscription_if_present,
};
use crate::stripe_webhook_processing::{
    BillingWebhookProcessingError, BillingWebhookProcessingResult,
    resolve_subscription_workspace_id,
};
use crate::stripe_webhook_validators_subscription as subscription;
use crate::stripe_webhook_workspace_effects::{
    apply_subscription_deleted_workspace_effects, apply_subscription_upsert_workspace_effects,
};
use crate::{required_string, stripe_subscription_status, timestamp_field};

pub async fn process_subscription_upsert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> BillingWebhookProcessingResult<Option<Uuid>> {
    subscription::ensure_subscription_update_consistency(object)?;
    let stripe_subscription_id = required_string(object, "id").ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "webhook_missing_id",
            "Subscription is missing id.",
        )
    })?;
    let workspace_id = workspace_id_for_stripe_subscription(tx, &stripe_subscription_id).await?;
    resolve_subscription_workspace_id(object, workspace_id)?;
    let subscription_context =
        provider_subscription_context(tx, workspace_id, &stripe_subscription_id).await?;
    subscription::ensure_subscription_item_consistency(object)?;
    subscription::ensure_subscription_price_consistency(object)?;
    subscription::ensure_subscription_quantity_consistency(object)?;
    let status = stripe_subscription_status(object.get("status").and_then(Value::as_str));
    let current_period_start = timestamp_field(object, "current_period_start");
    let current_period_end = timestamp_field(object, "current_period_end");
    persist_stripe_payment_method_if_present(
        tx,
        workspace_id,
        object,
        subscription_context.primary_for_subscription,
    )
    .await?;
    persist_stripe_subscription_if_present(
        tx,
        workspace_id,
        object,
        &stripe_subscription_id,
        status,
        current_period_start,
        current_period_end,
        subscription_context.primary_for_subscription,
        "subscription_upsert",
    )
    .await?;

    crate::db::insert_billing_audit_tx(tx, workspace_id, "billing.updated", object.clone()).await?;
    apply_subscription_upsert_workspace_effects(
        tx,
        workspace_id,
        &stripe_subscription_id,
        status,
        current_period_start,
        current_period_end,
        object,
        subscription_context.primary_for_subscription,
    )
    .await?;

    Ok(Some(workspace_id))
}

pub async fn process_subscription_deleted(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> BillingWebhookProcessingResult<Option<Uuid>> {
    let stripe_subscription_id = required_string(object, "id").ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "webhook_missing_id",
            "Subscription is missing id.",
        )
    })?;
    let workspace_id = workspace_id_for_stripe_subscription(tx, &stripe_subscription_id).await?;
    resolve_subscription_workspace_id(object, workspace_id)?;
    let subscription_context =
        provider_subscription_context(tx, workspace_id, &stripe_subscription_id).await?;
    subscription::ensure_subscription_deleted_consistency(object, workspace_id)?;
    persist_stripe_subscription_if_present(
        tx,
        workspace_id,
        object,
        &stripe_subscription_id,
        "canceled",
        timestamp_field(object, "current_period_start"),
        timestamp_field(object, "current_period_end"),
        false,
        "subscription_deleted",
    )
    .await?;
    crate::db::insert_billing_audit_tx(
        tx,
        workspace_id,
        "billing.subscription_deleted",
        object.clone(),
    )
    .await?;
    apply_subscription_deleted_workspace_effects(
        tx,
        workspace_id,
        object,
        subscription_context.primary_for_subscription,
    )
    .await?;
    Ok(Some(workspace_id))
}

async fn workspace_id_for_stripe_subscription(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subscription_id: &str,
) -> BillingWebhookProcessingResult<Uuid> {
    crate::db::workspace_id_for_provider_subscription_tx(tx, ProviderCode::Stripe, subscription_id)
        .await?
        .ok_or_else(|| {
            BillingWebhookProcessingError::bad_request(
                "unknown_provider_subscription",
                "Provider subscription is not mapped to a workspace.",
            )
        })
}

async fn provider_subscription_context(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    subscription_id: &str,
) -> BillingWebhookProcessingResult<crate::db::ProviderSubscriptionContext> {
    crate::db::provider_subscription_context_tx(
        tx,
        workspace_id,
        ProviderCode::Stripe,
        subscription_id,
    )
    .await?
    .ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "unknown_provider_subscription",
            "Provider subscription is not mapped to the workspace.",
        )
    })
}
