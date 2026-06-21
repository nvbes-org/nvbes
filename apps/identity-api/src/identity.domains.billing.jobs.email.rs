use crate::domains::billing::db;
use crate::email::jobs::{EmailSendPayload, enqueue_email_job_tx};
use crate::email::templates::html_escape;
use crate::http::error::AppError;
use nvbes_billing::StripeWebhookEvent;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn enqueue_billing_email_for_stripe_event(
    db_pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
    event: &StripeWebhookEvent,
) -> Result<(), AppError> {
    let Some(payload) = billing_email_payload(db_pool, workspace_id, event).await? else {
        return Ok(());
    };
    let idempotency_key = billing_email_idempotency_key(&event.id, &payload.business_type);
    enqueue_email_job_tx(db_pool, redis, payload, &idempotency_key).await
}

async fn billing_email_payload(
    db_pool: &PgPool,
    workspace_id: Uuid,
    event: &StripeWebhookEvent,
) -> Result<Option<EmailSendPayload>, AppError> {
    let billing = db::fetch_billing_state_pool(db_pool, workspace_id).await?;
    let to_email = billing
        .billing_email
        .clone()
        .unwrap_or_else(|| billing.owner_email.clone());
    let to_name = Some(billing.owner_email.clone());

    let payload = match event.event_type.as_str() {
        "invoice.payment_failed" => payment_failed_payload(&to_email, to_name, &event.data_object),
        "invoice.payment_succeeded" => receipt_payload(&to_email, to_name, &event.data_object),
        _ => None,
    };
    Ok(payload)
}

fn payment_failed_payload(
    to_email: &str,
    to_name: Option<String>,
    object: &Value,
) -> Option<EmailSendPayload> {
    let amount_due = object
        .get("amount_due")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let currency = invoice_currency(object);
    let hosted_invoice_url = object
        .get("hosted_invoice_url")
        .and_then(Value::as_str)
        .unwrap_or("");
    let subject = "Payment failed for your nvbes subscription".to_string();
    let text_body = format!(
        "We could not collect your nvbes subscription payment of {} {}. Update your payment method from your billing portal.{}",
        amount_due,
        currency,
        optional_invoice_text(hosted_invoice_url)
    );
    let html_body = format!(
        "<p>We could not collect your nvbes subscription payment of <strong>{} {}</strong>.</p><p>Update your payment method from your billing portal.</p>{}",
        amount_due,
        html_escape(&currency),
        optional_invoice_link(hosted_invoice_url)
    );

    Some(EmailSendPayload {
        to_email: to_email.to_string(),
        to_name,
        subject,
        html_body,
        text_body: Some(text_body),
        business_type: "billing_payment_failed".to_string(),
    })
}

fn receipt_payload(
    to_email: &str,
    to_name: Option<String>,
    object: &Value,
) -> Option<EmailSendPayload> {
    let amount_paid = object
        .get("amount_paid")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let currency = invoice_currency(object);
    let hosted_invoice_url = object
        .get("hosted_invoice_url")
        .and_then(Value::as_str)
        .unwrap_or("");
    let subject = "Receipt for your nvbes subscription".to_string();
    let text_body = format!(
        "We received your nvbes subscription payment of {} {}.{}",
        amount_paid,
        currency,
        optional_invoice_text(hosted_invoice_url)
    );
    let html_body = format!(
        "<p>We received your nvbes subscription payment of <strong>{} {}</strong>.</p>{}",
        amount_paid,
        html_escape(&currency),
        optional_invoice_link(hosted_invoice_url)
    );

    Some(EmailSendPayload {
        to_email: to_email.to_string(),
        to_name,
        subject,
        html_body,
        text_body: Some(text_body),
        business_type: "billing_receipt".to_string(),
    })
}

fn invoice_currency(object: &Value) -> String {
    object
        .get("currency")
        .and_then(Value::as_str)
        .unwrap_or("eur")
        .to_uppercase()
}

fn optional_invoice_text(hosted_invoice_url: &str) -> String {
    if hosted_invoice_url.is_empty() {
        String::new()
    } else {
        format!("\n\nInvoice: {hosted_invoice_url}")
    }
}

fn optional_invoice_link(hosted_invoice_url: &str) -> String {
    if hosted_invoice_url.is_empty() {
        String::new()
    } else {
        format!(
            "<p><a href=\"{}\">View invoice</a></p>",
            html_escape(hosted_invoice_url)
        )
    }
}

fn billing_email_idempotency_key(provider_event_id: &str, business_type: &str) -> String {
    format!("billing-email:{business_type}:{provider_event_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn billing_email_payment_failed_is_idempotent_per_provider_event() {
        assert_eq!(
            billing_email_idempotency_key("evt_1", "billing_payment_failed"),
            "billing-email:billing_payment_failed:evt_1"
        );
    }

    #[test]
    fn billing_email_payment_failed_includes_amount_and_escapes_url() {
        let payload = payment_failed_payload(
            "billing@example.com",
            None,
            &json!({
                "amount_due": 4680,
                "currency": "eur",
                "hosted_invoice_url": "https://example.com/invoice?x=<bad>",
            }),
        )
        .expect("payload");

        assert_eq!(payload.business_type, "billing_payment_failed");
        assert!(payload.text_body.unwrap().contains("4680 EUR"));
        assert!(payload.html_body.contains("4680 EUR"));
        assert!(payload.html_body.contains("&lt;bad&gt;"));
    }

    #[test]
    fn billing_email_receipt_uses_receipt_business_type() {
        let payload = receipt_payload(
            "billing@example.com",
            None,
            &json!({
                "amount_paid": 4680,
                "currency": "eur",
            }),
        )
        .expect("payload");

        assert_eq!(payload.business_type, "billing_receipt");
        assert!(payload.html_body.contains("4680 EUR"));
    }
}
