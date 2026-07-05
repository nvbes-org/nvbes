use serde_json::Value;
use uuid::Uuid;

use crate::provider::ProviderCode;
use crate::stripe_webhook_persistence::{
    persist_stripe_invoice_if_present, persist_stripe_payment_method_if_present,
    persist_stripe_subscription_if_present,
};
use crate::stripe_webhook_processing::{
    BillingWebhookProcessingError, BillingWebhookProcessingResult,
};
use crate::stripe_webhook_validators_invoice as invoice;
use crate::stripe_webhook_validators_workspace as workspace;
use crate::stripe_webhook_workspace_effects::{
    apply_invoice_payment_failed_workspace_effects,
    apply_invoice_payment_succeeded_workspace_effects,
};
use crate::{required_string, stripe_invoice_payment_failed_signal};

pub async fn process_invoice_payment_failed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> BillingWebhookProcessingResult<Option<Uuid>> {
    invoice::ensure_invoice_failure_consistency(object)?;
    invoice::ensure_invoice_billing_reason_consistency(object)?;
    invoice::ensure_invoice_payment_intent_consistency(object)?;
    invoice::ensure_invoice_amount_consistency(object)?;
    let subscription_id = required_string(object, "subscription").ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "webhook_missing_subscription",
            "Invoice is missing subscription id.",
        )
    })?;
    let workspace_id = workspace_id_for_stripe_subscription(tx, &subscription_id).await?;
    let subscription_context =
        provider_subscription_context(tx, workspace_id, &subscription_id).await?;
    invoice::ensure_invoice_subscription_consistency(
        object,
        Some(subscription_context.provider_subscription_id.as_str()),
        subscription_context.status.as_deref(),
    )?;
    workspace::ensure_invoice_workspace_consistency(object, workspace_id)?;
    persist_stripe_invoice_if_present(tx, workspace_id, object).await?;
    persist_stripe_subscription_if_present(
        tx,
        workspace_id,
        object,
        &subscription_id,
        "past_due",
        None,
        None,
        subscription_context.primary_for_subscription,
        "invoice_payment_failed",
    )
    .await?;
    let attempt_count = object
        .get("attempt_count")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    apply_invoice_payment_failed_workspace_effects(
        tx,
        workspace_id,
        &subscription_id,
        attempt_count,
        object,
        subscription_context.primary_for_subscription,
    )
    .await?;

    let fraud_signal = stripe_invoice_payment_failed_signal(object);
    crate::db::insert_billing_fraud_psp_signal_tx(tx, workspace_id, &fraud_signal).await?;
    crate::db::insert_billing_audit_tx(tx, workspace_id, "billing.payment_failed", object.clone())
        .await?;
    Ok(Some(workspace_id))
}

pub async fn process_invoice_payment_succeeded(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> BillingWebhookProcessingResult<Option<Uuid>> {
    invoice::ensure_invoice_payment_success_consistency(object)?;
    let subscription_id = required_string(object, "subscription").ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "webhook_missing_subscription",
            "Invoice is missing subscription id.",
        )
    })?;
    let workspace_id = workspace_id_for_stripe_subscription(tx, &subscription_id).await?;
    let subscription_context =
        provider_subscription_context(tx, workspace_id, &subscription_id).await?;
    workspace::ensure_invoice_workspace_consistency(object, workspace_id)?;
    persist_stripe_invoice_if_present(tx, workspace_id, object).await?;
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
        &subscription_id,
        "active",
        None,
        None,
        subscription_context.primary_for_subscription,
        "invoice_payment_succeeded",
    )
    .await?;

    apply_invoice_payment_succeeded_workspace_effects(
        tx,
        workspace_id,
        &subscription_id,
        object,
        subscription_context.primary_for_subscription,
    )
    .await?;

    crate::db::insert_billing_audit_tx(
        tx,
        workspace_id,
        "billing.payment_succeeded",
        object.clone(),
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
