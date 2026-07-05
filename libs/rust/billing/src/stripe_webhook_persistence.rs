use chrono::{TimeZone, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::db::{ProviderInvoiceInput, ProviderInvoiceLineInput};
use crate::provider::ProviderCode;
use crate::stripe_payment_method_from_object;
use crate::stripe_webhook_processing::BillingWebhookProcessingResult;

pub async fn persist_stripe_payment_method_if_present(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    object: &Value,
    primary_for_subscription: bool,
) -> BillingWebhookProcessingResult<()> {
    let Some(customer_id) = stripe_customer_id(object) else {
        return Ok(());
    };
    let Some(payment_method) = stripe_payment_method_from_object(object) else {
        return Ok(());
    };

    if primary_for_subscription {
        crate::db::upsert_provider_customer_tx(
            tx,
            workspace_id,
            ProviderCode::Stripe,
            &customer_id,
        )
        .await?;
    } else {
        crate::db::upsert_provider_customer_mapping_tx(
            tx,
            workspace_id,
            ProviderCode::Stripe,
            &customer_id,
        )
        .await?;
    }
    crate::db::upsert_provider_payment_method_tx(
        tx,
        ProviderCode::Stripe,
        &customer_id,
        &payment_method,
    )
    .await?;
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Stripe webhook projection carries provider and period state."
)]
pub async fn persist_stripe_subscription_if_present(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    object: &Value,
    provider_subscription_id: &str,
    status: &str,
    current_period_start: Option<chrono::DateTime<Utc>>,
    current_period_end: Option<chrono::DateTime<Utc>>,
    primary_for_subscription: bool,
    event_name: &str,
) -> BillingWebhookProcessingResult<()> {
    let Some(customer_id) = crate::required_string(object, "customer") else {
        return Ok(());
    };

    if primary_for_subscription {
        crate::db::upsert_provider_customer_tx(
            tx,
            workspace_id,
            ProviderCode::Stripe,
            &customer_id,
        )
        .await?;
    } else {
        crate::db::upsert_provider_customer_mapping_tx(
            tx,
            workspace_id,
            ProviderCode::Stripe,
            &customer_id,
        )
        .await?;
    }
    crate::db::upsert_provider_subscription_tx(
        tx,
        workspace_id,
        ProviderCode::Stripe,
        &customer_id,
        provider_subscription_id,
        status,
        current_period_start,
        current_period_end,
        primary_for_subscription,
        serde_json::json!({ "event": event_name }),
    )
    .await?;
    Ok(())
}

pub async fn persist_stripe_invoice_if_present(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    object: &Value,
) -> BillingWebhookProcessingResult<()> {
    let Some(input) = stripe_invoice_input(workspace_id, object) else {
        return Ok(());
    };
    crate::db::upsert_provider_invoice_tx(tx, input).await?;
    Ok(())
}

fn stripe_invoice_input(workspace_id: Uuid, object: &Value) -> Option<ProviderInvoiceInput> {
    let provider_invoice_id = string_field(object, "id")?;
    let currency = string_field(object, "currency")
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| "EUR".to_string());
    let total_minor = integer_field(object, "total")
        .or_else(|| integer_field(object, "amount_paid"))
        .or_else(|| integer_field(object, "amount_due"))
        .unwrap_or(0);
    let subtotal_minor = integer_field(object, "subtotal").unwrap_or(total_minor);
    let tax_minor = integer_field(object, "tax")
        .or_else(|| total_tax_amounts(object))
        .unwrap_or(0);

    Some(ProviderInvoiceInput {
        workspace_id,
        provider: ProviderCode::Stripe,
        provider_invoice_id,
        provider_invoice_number: string_field(object, "number"),
        provider_pdf_url: string_field(object, "invoice_pdf")
            .or_else(|| string_field(object, "hosted_invoice_url")),
        status: stripe_invoice_status(object.get("status").and_then(Value::as_str)).to_string(),
        currency: currency.clone(),
        subtotal_minor,
        tax_minor,
        total_minor,
        issued_at: timestamp_field(object, "created")
            .or_else(|| timestamp_field(object, "issued_at")),
        due_at: timestamp_field(object, "due_date"),
        paid_at: object
            .pointer("/status_transitions/paid_at")
            .and_then(Value::as_i64)
            .and_then(unix_timestamp),
        metadata: serde_json::json!({
            "provider": "stripe",
            "billing_reason": object.get("billing_reason").and_then(Value::as_str),
            "hosted_invoice_url": object.get("hosted_invoice_url").and_then(Value::as_str),
        }),
        lines: stripe_invoice_lines(object, &currency),
    })
}

fn stripe_invoice_status(status: Option<&str>) -> &'static str {
    match status {
        Some("draft") => "draft",
        Some("paid") => "paid",
        Some("void") => "void",
        Some("uncollectible") => "written_off",
        Some("open" | "past_due") => "issued",
        _ => "issued",
    }
}

fn stripe_invoice_lines(object: &Value, fallback_currency: &str) -> Vec<ProviderInvoiceLineInput> {
    let Some(lines) = object.pointer("/lines/data").and_then(Value::as_array) else {
        return Vec::new();
    };

    lines
        .iter()
        .map(|line| stripe_invoice_line(line, fallback_currency))
        .collect()
}

fn stripe_invoice_line(line: &Value, fallback_currency: &str) -> ProviderInvoiceLineInput {
    let amount_minor = integer_field(line, "amount").unwrap_or(0);
    let quantity = line
        .get("quantity")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .unwrap_or(1);
    let unit_amount_minor = line
        .pointer("/price/unit_amount")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| amount_minor / quantity.max(1));
    let currency = string_field(line, "currency")
        .map(|value| value.to_uppercase())
        .unwrap_or_else(|| fallback_currency.to_string());

    ProviderInvoiceLineInput {
        provider_line_id: string_field(line, "id"),
        line_type: string_field(line, "type").unwrap_or_else(|| "subscription".to_string()),
        description: string_field(line, "description")
            .unwrap_or_else(|| "Subscription".to_string()),
        quantity,
        currency,
        unit_amount_minor,
        amount_minor,
        tax_minor: total_tax_amounts(line).unwrap_or(0),
        metadata: serde_json::json!({
            "period_start": line.pointer("/period/start").and_then(Value::as_i64),
            "period_end": line.pointer("/period/end").and_then(Value::as_i64),
            "price_id": line.pointer("/price/id").and_then(Value::as_str),
        }),
    }
}

fn stripe_customer_id(object: &Value) -> Option<String> {
    match object.get("customer")? {
        Value::String(customer_id) => Some(customer_id.clone()),
        Value::Object(_) => object
            .pointer("/customer/id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

fn string_field(object: &Value, field: &str) -> Option<String> {
    object.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn integer_field(object: &Value, field: &str) -> Option<i64> {
    object.get(field).and_then(Value::as_i64)
}

fn timestamp_field(object: &Value, field: &str) -> Option<chrono::DateTime<Utc>> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .and_then(unix_timestamp)
}

fn unix_timestamp(value: i64) -> Option<chrono::DateTime<Utc>> {
    Utc.timestamp_opt(value, 0).single()
}

fn total_tax_amounts(object: &Value) -> Option<i64> {
    let values = object
        .get("total_tax_amounts")
        .and_then(Value::as_array)
        .or_else(|| object.get("tax_amounts").and_then(Value::as_array))?;
    Some(
        values
            .iter()
            .filter_map(|value| value.get("amount").and_then(Value::as_i64))
            .sum(),
    )
}
