use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    audit::record_audit_event,
    error::{BillingError, BillingResult},
    outbox::record_outbox_event,
};

pub async fn apply_subscription_event(
    db: &PgPool,
    event_id: &str,
    event_type: &str,
    event_created: DateTime<Utc>,
    data: &Value,
) -> BillingResult<()> {
    let mut transaction = db.begin().await?;
    apply_subscription_event_on_connection(
        &mut transaction,
        event_id,
        event_type,
        event_created,
        data,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn apply_subscription_event_on_connection(
    db: &mut PgConnection,
    event_id: &str,
    event_type: &str,
    event_created: DateTime<Utc>,
    data: &Value,
) -> BillingResult<()> {
    let sub_id = match data.get("id").and_then(Value::as_str) {
        Some(id) if id.starts_with("sub_") => id,
        _ => {
            // Might be an invoice event; check invoice.subscription
            match data
                .get("subscription")
                .or_else(|| data.pointer("/parent/subscription_details/subscription"))
                .and_then(Value::as_str)
            {
                Some(id) if id.starts_with("sub_") => id,
                _ => return Ok(()),
            }
        }
    };

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(sub_id)
        .execute(&mut *db)
        .await?;

    let customer_id = data
        .get("customer")
        .and_then(Value::as_str)
        .ok_or(BillingError::Invalid("missing_customer_id"))?;

    let account_info: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT account_id, account_type, email FROM billing_customers WHERE stripe_customer_id = $1",
    )
    .bind(customer_id)
    .fetch_optional(&mut *db)
    .await?;

    let (account_id, account_type, customer_email_db) = match account_info {
        Some(info) => info,
        None => {
            // Check metadata in object
            let meta_acc = data
                .pointer("/metadata/account_id")
                .and_then(Value::as_str)
                .and_then(|s| Uuid::parse_str(s).ok());
            match meta_acc {
                Some(acc) => (acc, "team".to_string(), None),
                None => return Ok(()),
            }
        }
    };

    let existing: Option<(Option<DateTime<Utc>>, String)> = sqlx::query_as(
        "SELECT last_event_created, status FROM billing_subscriptions WHERE stripe_subscription_id = $1",
    )
    .bind(sub_id)
    .fetch_optional(&mut *db)
    .await?;

    if let Some((Some(last_created), _)) = existing
        && last_created >= event_created
    {
        // Out of order: existing record was updated by a newer event, skip updating state
        return Ok(());
    }

    let raw_status = match event_type {
        "customer.subscription.deleted" => "canceled",
        "invoice.payment_failed" => "past_due",
        "invoice.paid" => "active",
        _ => data
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("active"),
    };

    let mapped_status = nvbes_billing::stripe::stripe_subscription_status(Some(raw_status));

    let plan_code = data
        .pointer("/metadata/plan_code")
        .and_then(Value::as_str)
        .or_else(|| {
            data.pointer("/items/data/0/price/lookup_key")
                .and_then(Value::as_str)
        })
        .unwrap_or("standard_monthly");

    let period_start = data
        .get("current_period_start")
        .and_then(Value::as_i64)
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single());
    let period_end = data
        .get("current_period_end")
        .and_then(Value::as_i64)
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single());
    let cancel_at_end = data
        .get("cancel_at_period_end")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    sqlx::query(
        r#"
        INSERT INTO billing_subscriptions (
            account_id, account_type, stripe_subscription_id, stripe_customer_id,
            plan_code, status, current_period_start, current_period_end,
            cancel_at_period_end, last_event_id, last_event_created
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (stripe_subscription_id) DO UPDATE SET
            plan_code = EXCLUDED.plan_code,
            status = EXCLUDED.status,
            current_period_start = EXCLUDED.current_period_start,
            current_period_end = EXCLUDED.current_period_end,
            cancel_at_period_end = EXCLUDED.cancel_at_period_end,
            last_event_id = EXCLUDED.last_event_id,
            last_event_created = EXCLUDED.last_event_created,
            updated_at = clock_timestamp()
        "#,
    )
    .bind(account_id)
    .bind(&account_type)
    .bind(sub_id)
    .bind(customer_id)
    .bind(plan_code)
    .bind(mapped_status)
    .bind(period_start)
    .bind(period_end)
    .bind(cancel_at_end)
    .bind(event_id)
    .bind(event_created)
    .execute(&mut *db)
    .await?;

    record_audit_event(
        &mut *db,
        account_id,
        "stripe_webhook",
        event_type,
        &json!({
            "subscription_id": sub_id,
            "status": mapped_status,
            "plan_code": plan_code,
        }),
    )
    .await?;

    record_outbox_event(
        &mut *db,
        "billing.subscription.changed.v1",
        account_id,
        &json!({
            "tenant_id": account_id,
            "subscription_id": sub_id,
            "status": mapped_status,
            "effective_at": event_created,
            "change_type": event_type,
        }),
    )
    .await?;

    if event_type == "invoice.paid" {
        let invoice_id = data.get("id").and_then(Value::as_str).unwrap_or(event_id);
        let amount_paid = data.get("amount_paid").and_then(Value::as_i64).unwrap_or(0);
        let currency = data
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("eur")
            .to_uppercase();
        let hosted_invoice_url = data
            .get("hosted_invoice_url")
            .and_then(Value::as_str)
            .map(str::to_string);
        let email = data
            .get("customer_email")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(customer_email_db.clone());
        let name = data
            .get("customer_name")
            .and_then(Value::as_str)
            .map(str::to_string);

        record_outbox_event(
            &mut *db,
            "billing.invoice.paid.v1",
            account_id,
            &json!({
                "account_id": account_id,
                "invoice_id": invoice_id,
                "amount_minor": amount_paid,
                "currency": currency,
                "recipient_email": email,
                "customer_name": name,
                "invoice_url": hosted_invoice_url,
            }),
        )
        .await?;
    } else if event_type == "invoice.payment_failed" {
        let invoice_id = data.get("id").and_then(Value::as_str).unwrap_or(event_id);
        let amount_due = data.get("amount_due").and_then(Value::as_i64).unwrap_or(0);
        let currency = data
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("eur")
            .to_uppercase();
        let hosted_invoice_url = data
            .get("hosted_invoice_url")
            .and_then(Value::as_str)
            .map(str::to_string);
        let email = data
            .get("customer_email")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(customer_email_db);
        let name = data
            .get("customer_name")
            .and_then(Value::as_str)
            .map(str::to_string);

        record_outbox_event(
            &mut *db,
            "billing.invoice.payment_failed.v1",
            account_id,
            &json!({
                "account_id": account_id,
                "invoice_id": invoice_id,
                "amount_minor": amount_due,
                "currency": currency,
                "recipient_email": email,
                "customer_name": name,
                "invoice_url": hosted_invoice_url,
            }),
        )
        .await?;
    }

    Ok(())
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.subscriptions.tests.rs"]
mod tests;
