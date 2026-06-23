use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::super::db::{
    plan_id_by_code_tx, plan_id_for_stripe_price_tx, project_workspace_plan_tx,
    upsert_billing_customer_tx, workspace_id_for_customer_tx,
};
use crate::http::error::AppError;
use nvbes_billing::{
    StripeWebhookEvent, metadata_workspace_id, parse_uuid, required_string,
    stripe_subscription_status, timestamp_field,
};

pub struct ProcessedBillingAnalytics {
    pub workspace_id: Uuid,
    pub event_name: &'static str,
    pub status: Option<&'static str>,
}

pub async fn process_stripe_event(
    tx: &mut Transaction<'_, Postgres>,
    event: &StripeWebhookEvent,
) -> Result<Option<ProcessedBillingAnalytics>, AppError> {
    let event_type = event.event_type.as_str();
    let data_object = &event.data_object;
    match event_type {
        "checkout.session.completed" => process_checkout_completed(tx, data_object).await,
        "customer.subscription.created" | "customer.subscription.updated" => {
            process_subscription_upsert(tx, data_object).await
        }
        "customer.subscription.deleted" => process_subscription_deleted(tx, data_object).await,
        "invoice.payment_failed" => process_invoice_payment_failed(tx, data_object).await,
        _ => Ok(None),
    }
}

async fn process_checkout_completed(
    tx: &mut Transaction<'_, Postgres>,
    object: &Value,
) -> Result<Option<ProcessedBillingAnalytics>, AppError> {
    let workspace_id = metadata_workspace_id(object)
        .or_else(|| {
            object
                .get("client_reference_id")
                .and_then(Value::as_str)
                .and_then(parse_uuid)
        })
        .ok_or_else(|| {
            AppError::bad_request(
                "webhook_missing_workspace",
                "Checkout session is missing workspace metadata.",
            )
        })?;
    let customer_id = required_string(object, "customer").ok_or_else(|| {
        AppError::bad_request(
            "webhook_missing_customer",
            "Checkout session is missing customer id.",
        )
    })?;
    let subscription_id = object
        .get("subscription")
        .and_then(Value::as_str)
        .map(str::to_owned);

    upsert_billing_customer_tx(tx, workspace_id, &customer_id).await?;

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

    insert_billing_audit(tx, workspace_id, "billing.checkout_completed", object).await?;
    Ok(None)
}

async fn process_subscription_upsert(
    tx: &mut Transaction<'_, Postgres>,
    object: &Value,
) -> Result<Option<ProcessedBillingAnalytics>, AppError> {
    let workspace_id = if let Some(workspace_id) = metadata_workspace_id(object) {
        workspace_id
    } else {
        let customer_id = required_string(object, "customer").ok_or_else(|| {
            AppError::bad_request(
                "webhook_missing_customer",
                "Subscription is missing customer id.",
            )
        })?;
        workspace_id_for_customer_tx(tx, &customer_id).await?
    };
    let stripe_subscription_id = required_string(object, "id").ok_or_else(|| {
        AppError::bad_request("webhook_missing_id", "Subscription is missing id.")
    })?;
    let customer_id = required_string(object, "customer").ok_or_else(|| {
        AppError::bad_request(
            "webhook_missing_customer",
            "Subscription is missing customer id.",
        )
    })?;
    let stripe_price_id = object
        .pointer("/items/data/0/price/id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::bad_request("webhook_missing_price", "Subscription is missing price id.")
        })?;
    let plan_id = plan_id_for_stripe_price_tx(tx, stripe_price_id).await?;
    let status = stripe_subscription_status(object.get("status").and_then(Value::as_str));
    let current_period_start = timestamp_field(object, "current_period_start");
    let current_period_end = timestamp_field(object, "current_period_end");

    upsert_billing_customer_tx(tx, workspace_id, &customer_id).await?;

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET plan_id = $2,
            status = $3::subscription_status,
            billing_customer_id = $4,
            billing_subscription_id = $5,
            current_period_start = $6,
            current_period_end = $7,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .bind(status)
    .bind(customer_id)
    .bind(stripe_subscription_id)
    .bind(current_period_start)
    .bind(current_period_end)
    .execute(tx.as_mut())
    .await?;

    project_workspace_plan_tx(tx, workspace_id, plan_id).await?;
    insert_billing_audit(tx, workspace_id, "billing.updated", object).await?;

    if matches!(status, "active" | "trialing") {
        Ok(Some(ProcessedBillingAnalytics {
            workspace_id,
            event_name: "billing.subscription_activated",
            status: Some(status),
        }))
    } else {
        Ok(None)
    }
}

async fn process_subscription_deleted(
    tx: &mut Transaction<'_, Postgres>,
    object: &Value,
) -> Result<Option<ProcessedBillingAnalytics>, AppError> {
    let workspace_id = if let Some(workspace_id) = metadata_workspace_id(object) {
        workspace_id
    } else {
        let customer_id = required_string(object, "customer").ok_or_else(|| {
            AppError::bad_request(
                "webhook_missing_customer",
                "Subscription is missing customer id.",
            )
        })?;
        workspace_id_for_customer_tx(tx, &customer_id).await?
    };
    let trial_plan_id = plan_id_by_code_tx(tx, "trial").await?;

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET plan_id = $2,
            status = 'canceled',
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(trial_plan_id)
    .execute(tx.as_mut())
    .await?;

    project_workspace_plan_tx(tx, workspace_id, trial_plan_id).await?;
    insert_billing_audit(tx, workspace_id, "billing.updated", object).await?;
    Ok(None)
}

async fn process_invoice_payment_failed(
    tx: &mut Transaction<'_, Postgres>,
    object: &Value,
) -> Result<Option<ProcessedBillingAnalytics>, AppError> {
    let customer_id = required_string(object, "customer").ok_or_else(|| {
        AppError::bad_request(
            "webhook_missing_customer",
            "Invoice payment failed event is missing customer id.",
        )
    })?;
    let workspace_id = workspace_id_for_customer_tx(tx, &customer_id).await?;

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'past_due',
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .execute(tx.as_mut())
    .await?;

    insert_billing_audit(tx, workspace_id, "billing.payment_failed", object).await?;
    Ok(Some(ProcessedBillingAnalytics {
        workspace_id,
        event_name: "billing.payment_failed",
        status: Some("past_due"),
    }))
}

async fn insert_billing_audit(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    action: &str,
    object: &Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(workspace_id)
    .bind(None::<Uuid>)
    .bind(None::<Uuid>)
    .bind(action)
    .bind("billing")
    .bind(workspace_id)
    .bind(None::<&str>)
    .bind(None::<&str>)
    .bind(sqlx::types::Json(object.clone()))
    .execute(tx.as_mut())
    .await?;

    Ok(())
}
