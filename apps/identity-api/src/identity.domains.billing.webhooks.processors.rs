use crate::http::error::AppError;
use nvbes_billing::{required_string, stripe_subscription_status, timestamp_field};
use serde_json::Value;
use sqlx::Row;

use super::super::db;
use super::logic::resolve_subscription_workspace_id;
use super::validators_checkout as checkout;
use super::validators_invoice as invoice;
use super::validators_subscription as subscription;
use super::validators_workspace as workspace;

pub async fn process_checkout_completed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> Result<Option<uuid::Uuid>, AppError> {
    let workspace_id = super::logic::resolve_webhook_workspace_id(object).ok_or_else(|| {
        AppError::bad_request(
            "webhook_missing_workspace",
            "Checkout session is missing workspace metadata.",
        )
    })?;
    workspace::ensure_checkout_workspace_consistency(object, workspace_id)?;
    let customer_id = required_string(object, "customer").ok_or_else(|| {
        AppError::bad_request(
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

    db::upsert_billing_customer_tx(tx, workspace_id, &customer_id).await?;

    if let Some(subscription_id) = subscription_id {
        sqlx::query(
            r#"
            UPDATE subscriptions
            SET billing_subscription_id = $2,
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

    db::insert_billing_audit(tx, workspace_id, "billing.checkout_completed", object).await?;
    Ok(Some(workspace_id))
}

pub async fn process_subscription_upsert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> Result<Option<uuid::Uuid>, AppError> {
    subscription::ensure_subscription_update_consistency(object)?;
    let stripe_subscription_id = required_string(object, "id").ok_or_else(|| {
        AppError::bad_request("webhook_missing_id", "Subscription is missing id.")
    })?;
    let workspace_id = db::workspace_id_for_subscription(tx, &stripe_subscription_id).await?;
    resolve_subscription_workspace_id(object, workspace_id)?;
    subscription::ensure_subscription_item_consistency(object)?;
    subscription::ensure_subscription_price_consistency(object)?;
    subscription::ensure_subscription_quantity_consistency(object)?;
    let stripe_price_id = object
        .pointer("/items/data/0/price/id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::bad_request("webhook_missing_price", "Subscription is missing price id.")
        })?;
    let plan_id = db::plan_id_for_stripe_price(tx, stripe_price_id).await?;
    let status = stripe_subscription_status(object.get("status").and_then(Value::as_str));
    let current_period_start = timestamp_field(object, "current_period_start");
    let current_period_end = timestamp_field(object, "current_period_end");

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET plan_id = $2,
            status = $3::subscription_status,
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
    .bind(stripe_subscription_id)
    .bind(current_period_start)
    .bind(current_period_end)
    .execute(tx.as_mut())
    .await?;

    db::project_workspace_plan(tx, workspace_id, plan_id).await?;
    db::insert_billing_audit(tx, workspace_id, "billing.updated", object).await?;

    Ok(Some(workspace_id))
}

pub async fn process_subscription_deleted(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> Result<Option<uuid::Uuid>, AppError> {
    let stripe_subscription_id = required_string(object, "id").ok_or_else(|| {
        AppError::bad_request("webhook_missing_id", "Subscription is missing id.")
    })?;
    let workspace_id = db::workspace_id_for_subscription(tx, &stripe_subscription_id).await?;
    resolve_subscription_workspace_id(object, workspace_id)?;
    subscription::ensure_subscription_deleted_consistency(object, workspace_id)?;
    let trial_plan_id = db::plan_id_by_code(tx, "trial").await?;
    db::insert_billing_audit(tx, workspace_id, "billing.subscription_deleted", object).await?;

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

    db::project_workspace_plan(tx, workspace_id, trial_plan_id).await?;
    Ok(Some(workspace_id))
}

pub async fn process_invoice_payment_failed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    object: &Value,
) -> Result<Option<uuid::Uuid>, AppError> {
    invoice::ensure_invoice_failure_consistency(object)?;
    invoice::ensure_invoice_billing_reason_consistency(object)?;
    invoice::ensure_invoice_payment_intent_consistency(object)?;
    invoice::ensure_invoice_amount_consistency(object)?;
    let subscription_id = required_string(object, "subscription").ok_or_else(|| {
        AppError::bad_request(
            "webhook_missing_subscription",
            "Invoice is missing subscription id.",
        )
    })?;
    let workspace_id = db::workspace_id_for_subscription(tx, &subscription_id).await?;
    let subscription_row = sqlx::query(
        r#"
        SELECT billing_subscription_id, status::text
        FROM subscriptions
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(tx.as_mut())
    .await?;
    let current_subscription_id: Option<String> =
        subscription_row.try_get("billing_subscription_id")?;
    let current_subscription_status: Option<String> = subscription_row.try_get("status")?;
    invoice::ensure_invoice_subscription_consistency(
        object,
        current_subscription_id.as_deref(),
        current_subscription_status.as_deref(),
    )?;
    workspace::ensure_invoice_workspace_consistency(object, workspace_id)?;

    if current_subscription_id.is_none() {
        sqlx::query(
            r#"
            UPDATE subscriptions
            SET billing_subscription_id = $2,
                updated_at = NOW()
            WHERE workspace_id = $1
              AND billing_subscription_id IS NULL
            "#,
        )
        .bind(workspace_id)
        .bind(subscription_id.as_str())
        .execute(tx.as_mut())
        .await?;
    }

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'past_due',
            billing_subscription_id = $2,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(&subscription_id)
    .execute(tx.as_mut())
    .await?;

    db::insert_billing_audit(tx, workspace_id, "billing.payment_failed", object).await?;
    Ok(Some(workspace_id))
}

pub async fn process_stripe_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &nvbes_billing::StripeWebhookEvent,
) -> Result<Option<uuid::Uuid>, AppError> {
    match event.event_type.as_str() {
        "checkout.session.completed" => process_checkout_completed(tx, &event.data_object).await,
        "customer.subscription.created" | "customer.subscription.updated" => {
            process_subscription_upsert(tx, &event.data_object).await
        }
        "customer.subscription.deleted" => {
            process_subscription_deleted(tx, &event.data_object).await
        }
        "invoice.payment_failed" => process_invoice_payment_failed(tx, &event.data_object).await,
        _ => Ok(None),
    }
}
