use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::provider::ProviderCode;
use crate::{
    StripeWebhookEvent, required_string, stripe_webhook_intake::resolve_webhook_workspace_id,
};

use super::stripe_webhook_persistence::persist_stripe_payment_method_if_present;
use super::stripe_webhook_processing_invoice::{
    process_invoice_payment_failed, process_invoice_payment_succeeded,
};
use super::stripe_webhook_processing_subscription::{
    process_subscription_deleted, process_subscription_upsert,
};
use super::stripe_webhook_validators_checkout as checkout;
use super::stripe_webhook_validators_workspace as workspace;

pub type BillingWebhookProcessingResult<T> = Result<T, BillingWebhookProcessingError>;

#[derive(Debug, Error)]
pub enum BillingWebhookProcessingError {
    #[error("{code}: {message}")]
    BadRequest {
        code: &'static str,
        message: &'static str,
    },
    #[error("{code}: {message}")]
    NotFound {
        code: &'static str,
        message: &'static str,
    },
    #[error("webhook processing database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl BillingWebhookProcessingError {
    pub fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self::BadRequest { code, message }
    }

    pub fn not_found(code: &'static str, message: &'static str) -> Self {
        Self::NotFound { code, message }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest { code, .. } | Self::NotFound { code, .. } => code,
            Self::Database(_) => "billing_webhook_processing_failed",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::BadRequest { message, .. } | Self::NotFound { message, .. } => message,
            Self::Database(_) => "Billing webhook processing failed.",
        }
    }
}

pub async fn process_stripe_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &StripeWebhookEvent,
) -> BillingWebhookProcessingResult<Option<Uuid>> {
    match event.event_type.as_str() {
        "checkout.session.completed" => process_checkout_completed(tx, &event.data_object).await,
        "customer.subscription.created" | "customer.subscription.updated" => {
            process_subscription_upsert(tx, &event.data_object).await
        }
        "customer.subscription.deleted" => {
            process_subscription_deleted(tx, &event.data_object).await
        }
        "invoice.payment_failed" => process_invoice_payment_failed(tx, &event.data_object).await,
        "invoice.payment_succeeded" => {
            process_invoice_payment_succeeded(tx, &event.data_object).await
        }
        _ => Ok(None),
    }
}

pub async fn process_checkout_completed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> BillingWebhookProcessingResult<Option<Uuid>> {
    let workspace_id = resolve_webhook_workspace_id(object).ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "webhook_missing_workspace",
            "Checkout session is missing workspace metadata.",
        )
    })?;
    workspace::ensure_checkout_workspace_consistency(object, workspace_id)?;
    let customer_id = required_string(object, "customer").ok_or_else(|| {
        BillingWebhookProcessingError::bad_request(
            "webhook_missing_customer",
            "Checkout session is missing customer id.",
        )
    })?;
    checkout::ensure_checkout_session_status_consistency(object)?;
    let subscription_id = object
        .get("subscription")
        .and_then(Value::as_str)
        .map(str::to_owned);
    checkout::ensure_checkout_mode_consistency(object, subscription_id.is_some())?;
    checkout::ensure_checkout_payment_status_consistency(object, subscription_id.is_some())?;
    checkout::ensure_checkout_subscription_presence(object, subscription_id.is_some())?;

    crate::db::upsert_provider_customer_tx(tx, workspace_id, ProviderCode::Stripe, &customer_id)
        .await?;
    persist_stripe_payment_method_if_present(tx, workspace_id, object, true).await?;

    if let Some(subscription_id) = subscription_id {
        sqlx::query(
            r#"
            UPDATE subscriptions
            SET billing_provider = 'stripe',
                billing_subscription_id = $2,
                billing_customer_id = $3,
                updated_at = NOW()
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .bind(subscription_id)
        .bind(customer_id)
        .execute(tx.as_mut())
        .await?;
    }

    crate::db::insert_billing_audit_tx(
        tx,
        workspace_id,
        "billing.checkout_completed",
        object.clone(),
    )
    .await?;
    Ok(Some(workspace_id))
}

pub(crate) fn resolve_subscription_workspace_id(
    object: &Value,
    mapped_workspace_id: Uuid,
) -> BillingWebhookProcessingResult<Uuid> {
    let workspace_id = resolve_webhook_workspace_id(object).unwrap_or(mapped_workspace_id);
    workspace::ensure_subscription_workspace_consistency(object, workspace_id)?;
    workspace::ensure_subscription_workspace_alignment(object, mapped_workspace_id)?;
    Ok(workspace_id)
}
